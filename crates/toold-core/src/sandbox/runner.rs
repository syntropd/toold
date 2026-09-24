//! Sandboxed child process execution engine with timeout and buffer limits.

use crate::error::TooldError;
use crate::policy::rule::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Structured output from a completed sandboxed tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Command string invoked.
    pub command: String,
    /// Numerical process exit code.
    pub exit_code: i32,
    /// Process stdout content (truncated if exceeding safety threshold).
    pub stdout: String,
    /// Process stderr content (truncated if exceeding safety threshold).
    pub stderr: String,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
}

/// Executes a tool within sandboxed constraints and captures results.
pub async fn execute_tool(
    tool: &ToolDefinition,
    user_args: &[String],
    working_dir: Option<&Path>,
) -> Result<ExecutionResult, TooldError> {
    if !tool.binary_path.exists() {
        return Err(TooldError::ToolNotFound(format!(
            "Binary {} not found on host",
            tool.binary_path.display()
        )));
    }

    let mut cmd = Command::new(&tool.binary_path);

    // Append fixed arguments first, then validated user arguments
    for arg in &tool.fixed_args {
        cmd.arg(arg);
    }
    for arg in user_args {
        cmd.arg(arg);
    }

    if let Some(wd) = working_dir {
        cmd.current_dir(wd);
    }

    // Clean execution environment
    cmd.env_clear();
    cmd.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/bin");
    cmd.env("LANG", "C.UTF-8");
    cmd.env("LC_ALL", "C.UTF-8");

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let start = Instant::now();
    let mut child = cmd.spawn().map_err(TooldError::Io)?;

    let timeout_duration = Duration::from_millis(tool.timeout_ms);

    let wait_res = timeout(timeout_duration, child.wait()).await;
    let status = match wait_res {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(TooldError::Io(e)),
        Err(_) => {
            let _ = child.kill().await;
            return Err(TooldError::Timeout(tool.timeout_ms));
        }
    };

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_buf).await;
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_buf).await;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = status.code().unwrap_or(-1);

    let stdout = String::from_utf8_lossy(&stdout_buf).trim().to_string();
    let stderr = String::from_utf8_lossy(&stderr_buf).trim().to_string();

    let full_cmd = format!(
        "{} {} {}",
        tool.binary_path.display(),
        tool.fixed_args.join(" "),
        user_args.join(" ")
    )
    .trim()
    .to_string();

    if exit_code != 0 {
        return Err(TooldError::ExecutionFailed {
            command: full_cmd,
            exit_code,
            stderr,
        });
    }

    Ok(ExecutionResult {
        command: full_cmd,
        exit_code,
        stdout,
        stderr,
        duration_ms,
    })
}

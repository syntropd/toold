//! Sandboxed child process execution engine with timeout and buffer limits.

use crate::error::TooldError;
use crate::policy::rule::ToolDefinition;
use crate::sandbox::landlock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncReadExt};
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

/// Drains an optional child pipe to EOF into a fresh buffer.
async fn drain_pipe<R>(pipe: &mut Option<R>) -> Result<Vec<u8>, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    if let Some(mut p) = pipe.take() {
        p.read_to_end(&mut buf).await?;
    }
    Ok(buf)
}

/// Executes a tool within sandboxed constraints and captures results.
pub async fn execute_tool(
    tool: &ToolDefinition,
    user_args: &[String],
    working_dir: Option<&Path>,
) -> Result<ExecutionResult, TooldError> {
    let bin_str = tool.binary_path.to_string_lossy();
    if bin_str.ends_with("/sh")
        || bin_str.ends_with("/bash")
        || bin_str.ends_with("/dash")
        || bin_str.ends_with("/zsh")
        || bin_str == "sh"
        || bin_str == "bash"
        || bin_str == "dash"
        || bin_str == "zsh"
    {
        return Err(TooldError::PermissionDenied(format!(
            "Shell binary {} rejected by zero-shell policy",
            bin_str
        )));
    }
    for arg in tool.fixed_args.iter().chain(user_args.iter()) {
        if arg == "-c"
            && (bin_str.contains("sh")
                || bin_str.contains("bash")
                || bin_str.contains("dash")
                || bin_str.contains("zsh"))
        {
            return Err(TooldError::PermissionDenied(
                "Shell flag -c rejected by zero-shell policy".into(),
            ));
        }
    }

    if tool.name == "net.socket_diag"
        || tool.binary_path.to_string_lossy() == "builtin:net.socket_diag"
    {
        let start = Instant::now();
        let stdout = crate::diag::run_socket_diag(user_args)?;
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(ExecutionResult {
            command: "net.socket_diag".into(),
            exit_code: 0,
            stdout,
            stderr: String::new(),
            duration_ms,
        });
    }

    if !tool.binary_path.exists() {
        return Err(TooldError::ToolNotFound(format!(
            "Binary {} not found on host",
            tool.binary_path.display()
        )));
    }

    let (mut cmd, uses_bwrap) =
        if let Some(bwrap_cmd) = super::bwrap::build_bwrap_command(tool, user_args, working_dir) {
            (bwrap_cmd, true)
        } else {
            let mut fallback = Command::new(&tool.binary_path);
            for arg in &tool.fixed_args {
                fallback.arg(arg);
            }
            for arg in user_args {
                fallback.arg(arg);
            }
            fallback.current_dir(working_dir.unwrap_or(Path::new("/run")));
            (fallback, false)
        };

    // Clean execution environment
    cmd.env_clear();
    cmd.env(
        "PATH",
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/bin",
    );
    cmd.env("LANG", "C.UTF-8");
    cmd.env("LC_ALL", "C.UTF-8");

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if !uses_bwrap {
        // Landlock confinement: ruleset built here in the parent
        let mut ruleset = Some(landlock::build_ruleset(tool)?);
        // SAFETY: pre_exec runs once per spawn, post-fork pre-exec; the
        // closure only takes the prebuilt ruleset and issues syscalls.
        unsafe {
            cmd.pre_exec(move || {
                let rs = ruleset
                    .take()
                    .ok_or_else(|| std::io::Error::other("landlock ruleset already applied"))?;
                landlock::restrict_child(rs).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
                })
            });
        }
    }

    let start = Instant::now();
    let mut child = cmd.spawn().map_err(TooldError::Io)?;
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    // Drain both pipes WHILE the child runs: waiting first deadlocks
    // once output exceeds the 64 KiB pipe buffer (every real journal).
    let run = async {
        let (status, out_buf, err_buf) = tokio::try_join!(
            child.wait(),
            drain_pipe(&mut stdout_pipe),
            drain_pipe(&mut stderr_pipe)
        )?;
        Ok::<_, std::io::Error>((status, out_buf, err_buf))
    };

    let timeout_duration = Duration::from_millis(tool.timeout_ms);
    let (status, stdout_buf, stderr_buf) = match timeout(timeout_duration, run).await {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => return Err(TooldError::Io(e)),
        Err(_) => {
            let _ = child.kill().await;
            return Err(TooldError::Timeout(tool.timeout_ms));
        }
    };

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

    if !tool.allowed_exit_codes.contains(&exit_code) {
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

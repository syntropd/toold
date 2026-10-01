//! Autonomous code self-correction loop with sandboxed execution and reflection.

use super::diagnostic::{parse_diagnostics, Diagnostic};
use super::runner::execute_tool;
use crate::error::TooldError;
use crate::policy::rule::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::tempdir;

pub use super::completer::ModelCompleter;
pub const MAX_ITERATIONS: usize = 5;

/// Output record for an autonomous code correction run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLoopResult {
    pub success: bool,
    pub iterations: usize,
    pub final_code: String,
    pub diagnostics: Vec<Diagnostic>,
    pub execution_stdout: String,
}

/// Runs the autonomous self-correction loop up to 5 iterations.
pub async fn run_self_correction_loop<M: ModelCompleter>(
    initial_code: &str,
    language: &str,
    model: &M,
) -> Result<CodeLoopResult, TooldError> {
    let mut current_code = initial_code.to_string();
    let mut last_diagnostics = Vec::new();
    let mut last_stdout = String::new();

    for iteration in 1..=MAX_ITERATIONS {
        let (exit_code, stdout, stderr) = execute_code_sandboxed(&current_code, language).await?;
        last_stdout = stdout;

        if exit_code == 0 {
            return Ok(CodeLoopResult {
                success: true,
                iterations: iteration,
                final_code: current_code,
                diagnostics: Vec::new(),
                execution_stdout: last_stdout,
            });
        }

        last_diagnostics = parse_diagnostics(language, &stderr);
        if iteration == MAX_ITERATIONS {
            break;
        }

        let prompt = build_reflection_prompt(&current_code, language, &stderr, &last_diagnostics);
        let completion = model.complete(&prompt)?;
        if let Some(patched) = extract_code_block(&completion, language) {
            current_code = patched;
        } else if !completion.trim().is_empty() {
            current_code = completion.trim().to_string();
        }
    }

    Ok(CodeLoopResult {
        success: false,
        iterations: MAX_ITERATIONS,
        final_code: current_code,
        diagnostics: last_diagnostics,
        execution_stdout: last_stdout,
    })
}

async fn execute_code_sandboxed(
    code: &str,
    language: &str,
) -> Result<(i32, String, String), TooldError> {
    let dir = tempdir().map_err(TooldError::Io)?;
    let ext = if language == "rust" || language == "rs" {
        "rs"
    } else {
        "py"
    };
    let file_path = dir.path().join(format!("scratch.{}", ext));
    fs::write(&file_path, code).map_err(TooldError::Io)?;

    let bin = if ext == "rs" {
        "/usr/bin/rustc"
    } else {
        "/usr/bin/python3"
    };
    let fixed_args = if ext == "rs" {
        let out_bin = dir.path().join("scratch_bin");
        vec![
            "--error-format=json".into(),
            "-o".into(),
            out_bin.to_string_lossy().to_string(),
            file_path.to_string_lossy().to_string(),
        ]
    } else {
        vec![file_path.to_string_lossy().to_string()]
    };

    let mut tool =
        ToolDefinition::read_only("scratch.exec", "Scratch execution", bin, fixed_args, 5000);
    tool.read_paths.push(dir.path().to_path_buf());
    tool.write_paths.push(dir.path().to_path_buf());
    tool.allowed_exit_codes = vec![0, 1, 2, 101];

    let (c_exit, c_out, c_err) = match execute_tool(&tool, &[], Some(dir.path())).await {
        Ok(res) => (res.exit_code, res.stdout, res.stderr),
        Err(TooldError::ExecutionFailed {
            exit_code, stderr, ..
        }) => (exit_code, String::new(), stderr),
        Err(e) => return Err(e),
    };
    if c_exit != 0 || ext != "rs" {
        return Ok((c_exit, c_out, c_err));
    }

    // If Rust compilation succeeded, execute the produced binary
    let out_bin = dir.path().join("scratch_bin");
    let mut run_tool =
        ToolDefinition::read_only("scratch.run", "Run scratch binary", &out_bin, vec![], 5000);
    run_tool.read_paths.push(dir.path().to_path_buf());
    run_tool.write_paths.push(dir.path().to_path_buf());
    run_tool.allowed_exit_codes = vec![0, 1, 2, 101];
    let (r_exit, r_out, r_err) = match execute_tool(&run_tool, &[], Some(dir.path())).await {
        Ok(res) => (res.exit_code, res.stdout, res.stderr),
        Err(TooldError::ExecutionFailed {
            exit_code, stderr, ..
        }) => (exit_code, String::new(), stderr),
        Err(e) => return Err(e),
    };
    Ok((r_exit, r_out, r_err))
}

fn build_reflection_prompt(
    code: &str,
    language: &str,
    stderr: &str,
    diags: &[Diagnostic],
) -> String {
    let diag_summary = if !diags.is_empty() {
        format!(
            "Parsed error: {} (line {:?})",
            diags[0].message, diags[0].line
        )
    } else {
        "Unknown error".into()
    };

    format!(
        "You are an autonomous compiler and runtime repair agent.\n\
         Fix the error in the following {} code.\n\n\
         CURRENT CODE:\n\
         ```{}\n\
         {}\n\
         ```\n\n\
         DIAGNOSTICS:\n\
         {}\n\n\
         STDERR:\n\
         {}\n\n\
         Respond with ONLY the corrected code inside a single ```{} ... ``` markdown block.",
        language, language, code, diag_summary, stderr, language
    )
}

fn extract_code_block(completion: &str, language: &str) -> Option<String> {
    let lang_marker = format!("```{}", language);
    let start = completion
        .find(&lang_marker)
        .map(|p| p + lang_marker.len())
        .or_else(|| completion.find("```").map(|p| p + 3))?;
    let rest = &completion[start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct MockRepairModel {
        fixed_code: String,
        lang: &'static str,
    }

    impl ModelCompleter for MockRepairModel {
        fn complete(&self, prompt: &str) -> Result<String, TooldError> {
            assert!(prompt.contains("CURRENT CODE:"));
            Ok(format!("```{}\n{}\n```", self.lang, self.fixed_code))
        }
    }

    #[tokio::test]
    async fn test_auto_repair_syntax_error() {
        if !Path::new("/usr/bin/python3").exists() {
            return;
        }
        let broken = "print(\"hello broken world\"";
        let fixed = "print(\"hello fixed world\")";
        let model = MockRepairModel {
            fixed_code: fixed.into(),
            lang: "python",
        };

        let res = run_self_correction_loop(broken, "python", &model)
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.iterations, 2);
        assert_eq!(res.final_code, fixed);
        assert!(res.execution_stdout.contains("hello fixed world"));
    }

    #[tokio::test]
    async fn test_auto_repair_rust_syntax_error() {
        if !Path::new("/usr/bin/rustc").exists() {
            return;
        }
        let broken = "fn main() { println!(\"broken\") ";
        let fixed = "fn main() { println!(\"fixed\"); }";
        let model = MockRepairModel {
            fixed_code: fixed.into(),
            lang: "rust",
        };
        let res = run_self_correction_loop(broken, "rust", &model)
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.iterations, 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_trips_after_five_iterations() {
        if !Path::new("/usr/bin/python3").exists() {
            return;
        }
        let broken = "invalid python code ???";
        let model = MockRepairModel {
            fixed_code: broken.into(),
            lang: "python",
        };

        let res = run_self_correction_loop(broken, "python", &model)
            .await
            .unwrap();
        assert!(!res.success);
        assert_eq!(res.iterations, MAX_ITERATIONS);
    }
}

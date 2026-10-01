//! Compiler and runtime diagnostic stderr parser for Rust and Python.

use serde::{Deserialize, Serialize};

/// Structured diagnostic captured from a failing compiler or runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub language: String,
    pub error_type: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub raw_snippet: Option<String>,
}

/// Parses error diagnostic information from compiler/runtime stderr.
pub fn parse_diagnostics(language: &str, stderr: &str) -> Vec<Diagnostic> {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => parse_rust_diagnostics(stderr),
        "python" | "py" => parse_python_diagnostics(stderr),
        _ => parse_generic_diagnostics(language, stderr),
    }
}

fn parse_rust_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let level = val.get("level").and_then(|v| v.as_str()).unwrap_or("error");
                if level == "error" {
                    let message = val
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let (line_num, col, snippet) = val
                        .get("spans")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .map(|span| {
                            let line = span
                                .get("line_start")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as usize);
                            let col = span
                                .get("column_start")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as usize);
                            let text = span
                                .get("text")
                                .and_then(|t| t.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|t| t.get("text"))
                                .and_then(|s| s.as_str())
                                .map(ToString::to_string);
                            (line, col, text)
                        })
                        .unwrap_or((None, None, None));

                    diags.push(Diagnostic {
                        language: "rust".into(),
                        error_type: "CompilerError".into(),
                        message,
                        line: line_num,
                        column: col,
                        raw_snippet: snippet,
                    });
                }
            }
        } else if trimmed.contains("panicked at") {
            let msg = if let Some(pos) = trimmed.find("panicked at") {
                trimmed[pos..].to_string()
            } else {
                trimmed.to_string()
            };
            diags.push(Diagnostic {
                language: "rust".into(),
                error_type: "Panic".into(),
                message: msg,
                line: None,
                column: None,
                raw_snippet: Some(trimmed.to_string()),
            });
        }
    }

    if diags.is_empty() {
        diags = parse_generic_diagnostics("rust", stderr);
    }
    diags
}

fn parse_python_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut current_line = None;
    let mut current_snippet = None;

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("File \"") && trimmed.contains(", line ") {
            if let Some(pos) = trimmed.find(", line ") {
                let rest = &trimmed[pos + 7..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    current_line = Some(n);
                }
            }
        } else if trimmed.ends_with("Error:") || trimmed.contains("Error: ") {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            let err_type = parts[0].trim().to_string();
            let msg = parts
                .get(1)
                .map(|m| m.trim().to_string())
                .unwrap_or_default();
            diags.push(Diagnostic {
                language: "python".into(),
                error_type: err_type,
                message: msg,
                line: current_line,
                column: None,
                raw_snippet: current_snippet.take(),
            });
        } else if !trimmed.is_empty() && !trimmed.starts_with("Traceback") {
            current_snippet = Some(trimmed.to_string());
        }
    }

    if diags.is_empty() {
        diags = parse_generic_diagnostics("python", stderr);
    }
    diags
}

fn parse_generic_diagnostics(language: &str, stderr: &str) -> Vec<Diagnostic> {
    let first_line = stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("Unknown error");
    vec![Diagnostic {
        language: language.into(),
        error_type: "RuntimeError".into(),
        message: first_line.trim().to_string(),
        line: None,
        column: None,
        raw_snippet: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_syntax_error() {
        let err = "  File \"scratch.py\", line 4\n    print(\"missing\"\n          ^\nSyntaxError: was never closed";
        let diags = parse_diagnostics("python", err);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, Some(4));
        assert_eq!(diags[0].error_type, "SyntaxError");
    }

    #[test]
    fn test_parse_rust_json_error() {
        let json_err = r#"{"message":"cannot find value `x`","level":"error","spans":[{"file_name":"test.rs","line_start":2,"column_start":5,"text":[{"text":"    x;"}]}]}"#;
        let diags = parse_diagnostics("rust", json_err);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, Some(2));
        assert_eq!(diags[0].column, Some(5));
        assert!(diags[0].message.contains("cannot find value `x`"));
    }
}

//! Error types for toold operations.

use thiserror::Error;

/// Error categories emitted across toold components.
#[derive(Debug, Error)]
pub enum TooldError {
    /// Action not allowed under active zero-trust security policy.
    #[error("Permission denied by policy: {0}")]
    PermissionDenied(String),

    /// Requested tool name was not registered.
    #[error("Tool not found in registry: {0}")]
    ToolNotFound(String),

    /// Execution terminated with a non-zero exit status.
    #[error("Tool execution failed (command: '{command}', exit: {exit_code}): {stderr}")]
    ExecutionFailed {
        /// Binary or command invoked.
        command: String,
        /// Numerical exit status code.
        exit_code: i32,
        /// Error diagnostic stream content.
        stderr: String,
    },

    /// Tool invocation exceeded its allotted runtime limit.
    #[error("Tool execution timed out after {0} ms")]
    Timeout(u64),

    /// Sandbox restriction violation or failure to apply Landlock.
    #[error("Sandbox security failure: {0}")]
    Sandbox(String),

    /// Rollback journal persistence or restoration failure.
    #[error("Rollback journal error: {0}")]
    Journal(String),

    /// Configuration parsing error.
    #[error("Configuration parse error: {0}")]
    Config(String),

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

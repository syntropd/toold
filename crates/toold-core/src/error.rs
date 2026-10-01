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

    /// Actuator hardware or uinput error.
    #[error("Actuator error: {0}")]
    Actuator(String),

    /// Socket or system diagnostic failure.
    #[error("Diagnostic error: {0}")]
    Diagnostic(String),

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        assert_eq!(
            TooldError::PermissionDenied("unit.reboot".into()).to_string(),
            "Permission denied by policy: unit.reboot"
        );
        assert_eq!(
            TooldError::ToolNotFound("nope.tool".into()).to_string(),
            "Tool not found in registry: nope.tool"
        );
        assert_eq!(
            TooldError::Timeout(200).to_string(),
            "Tool execution timed out after 200 ms"
        );
        assert_eq!(
            TooldError::Sandbox("landlock".into()).to_string(),
            "Sandbox security failure: landlock"
        );
        assert_eq!(
            TooldError::Journal("corrupt".into()).to_string(),
            "Rollback journal error: corrupt"
        );
        assert_eq!(
            TooldError::Config("bad toml".into()).to_string(),
            "Configuration parse error: bad toml"
        );
    }

    #[test]
    fn test_execution_failed_display_details() {
        let err = TooldError::ExecutionFailed {
            command: "/usr/bin/false".into(),
            exit_code: 1,
            stderr: "boom".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/usr/bin/false"));
        assert!(msg.contains("exit: 1"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = TooldError::from(io_err);
        assert!(matches!(err, TooldError::Io(_)));
        assert!(err.to_string().contains("I/O error"));
    }
}

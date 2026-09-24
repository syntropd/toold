//! Unit QA tests for security policy rules and definitions.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use toold_core::policy::{ExecutionMode, ToolDefinition};

    #[test]
    fn test_read_only_tool_creation() {
        let tool = ToolDefinition::read_only(
            "test.read",
            "Read-only test command",
            "/usr/bin/true",
            vec!["--version".into()],
            2000,
        );

        assert_eq!(tool.name, "test.read");
        assert_eq!(tool.mode, ExecutionMode::ReadOnly);
        assert_eq!(tool.timeout_ms, 2000);
        assert!(tool.write_paths.is_empty());
        assert!(!tool.read_paths.is_empty());
    }

    #[test]
    fn test_remediate_tool_creation() {
        let write_paths = vec![PathBuf::from("/etc/systemd/system")];
        let tool = ToolDefinition::remediate(
            "test.write",
            "Remediating test action",
            "/usr/bin/touch",
            vec!["foo".into()],
            5000,
            write_paths.clone(),
        );

        assert_eq!(tool.name, "test.write");
        assert_eq!(tool.mode, ExecutionMode::RemediateWithRollback);
        assert_eq!(tool.write_paths, write_paths);
    }

    #[test]
    fn test_execution_mode_serialization() {
        let mode = ExecutionMode::RemediateWithRollback;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"RemediateWithRollback\"");

        let deserialized: ExecutionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);
    }
}

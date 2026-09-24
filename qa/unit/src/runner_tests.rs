//! Unit QA tests for sandboxed tool execution engine.

#[cfg(test)]
mod tests {
    use toold_core::error::TooldError;
    use toold_core::policy::ToolDefinition;
    use toold_core::sandbox::execute_tool;

    #[tokio::test]
    async fn test_execute_echo_tool() {
        let tool = ToolDefinition::read_only(
            "test.echo",
            "Echo command",
            "/usr/bin/echo",
            vec!["fixed".into()],
            2000,
        );

        let user_args = vec!["arg1".to_string(), "arg2".to_string()];
        let res = execute_tool(&tool, &user_args, None).await.unwrap();

        assert_eq!(res.exit_code, 0);
        assert_eq!(res.stdout, "fixed arg1 arg2");
    }

    #[tokio::test]
    async fn test_execute_non_zero_exit_fails() {
        let tool = ToolDefinition::read_only(
            "test.false",
            "Failing command",
            "/usr/bin/false",
            vec![],
            2000,
        );

        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            TooldError::ExecutionFailed { exit_code, .. } => {
                assert_ne!(exit_code, 0);
            }
            other => panic!("Unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_execute_nonexistent_binary_fails() {
        let tool = ToolDefinition::read_only(
            "test.missing",
            "Missing binary",
            "/opt/nonexistent_bin_12345",
            vec![],
            2000,
        );

        let res = execute_tool(&tool, &[], None).await;
        assert!(matches!(res, Err(TooldError::ToolNotFound(_))));
    }
}

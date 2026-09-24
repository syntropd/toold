//! Edge tests for tool execution timeout enforcement.

#[cfg(test)]
mod tests {
    use toold_core::error::TooldError;
    use toold_core::policy::ToolDefinition;
    use toold_core::sandbox::execute_tool;

    #[tokio::test]
    async fn test_tool_timeout_enforced() {
        let tool = ToolDefinition::read_only(
            "test.sleep",
            "Long running sleep",
            "/usr/bin/sleep",
            vec!["5".into()],
            200, // 200 ms timeout
        );

        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), TooldError::Timeout(200)));
    }
}

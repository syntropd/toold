//! Edge tests for high-volume process stdout/stderr handling.

#[cfg(test)]
mod tests {
    use toold_core::policy::ToolDefinition;
    use toold_core::sandbox::execute_tool;

    #[tokio::test]
    async fn test_high_volume_stdout_handling() {
        let tool = ToolDefinition::read_only(
            "test.seq",
            "Generate many output lines",
            "/usr/bin/seq",
            vec!["1".into(), "10000".into()],
            5000,
        );

        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_ok());
        let output = res.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(!output.stdout.is_empty());
        assert!(output.stdout.contains("10000"));
    }
}

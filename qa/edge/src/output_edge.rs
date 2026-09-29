//! Edge tests for high-volume process stdout/stderr handling.

#[cfg(test)]
mod tests {
    use toold_core::policy::ToolDefinition;
    use toold_core::sandbox::execute_tool;

    #[tokio::test]
    async fn test_high_volume_stdout_handling() {
        // ~190 KiB: past the 64 KiB pipe buffer, so a wait-before-read
        // runner deadlocks here (every real journal trips this).
        let tool = ToolDefinition::read_only(
            "test.seq",
            "Generate many output lines",
            "/usr/bin/seq",
            vec!["1".into(), "30000".into()],
            15000,
        );

        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_ok());
        let output = res.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(
            output.stdout.len() > 65536,
            "short read: {}",
            output.stdout.len()
        );
        assert!(output.stdout.contains("30000"));
    }
}

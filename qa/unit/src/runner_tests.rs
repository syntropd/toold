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

    #[tokio::test]
    async fn test_syntax_verify_runs_confined() {
        // The real analyzer under the real profile: needs its /tmp
        // scratch grant (regresses without it). Skips off-systemd.
        if !std::path::Path::new("/usr/bin/systemd-analyze").exists() {
            return;
        }
        let registry = toold_core::policy::ToolRegistry::with_defaults();
        let tool = registry.get("syntax.verify").unwrap();
        let unit = if std::path::Path::new("/lib/systemd/system/systemd-journald.service").exists()
            || std::path::Path::new("/usr/lib/systemd/system/systemd-journald.service").exists()
        {
            "systemd-journald.service"
        } else {
            "routerd.service"
        };
        let res = execute_tool(&tool, &[unit.to_string()], None).await;
        assert!(res.is_ok(), "confined verify failed: {res:?}");
    }

    #[tokio::test]
    async fn test_child_cwd_defaults_inside_profile() {
        // CWD must resolve under Landlock (analyzers call getcwd).
        let tool = ToolDefinition::read_only("test.pwd", "Print CWD", "/usr/bin/pwd", vec![], 2000);
        let res = execute_tool(&tool, &[], None).await.unwrap();
        assert_eq!(res.stdout, "/run");
    }

    #[tokio::test]
    async fn test_landlock_denies_writes_outside_profile() {
        // /tmp is outside every tool profile: the write must fail and
        // no file may appear. Passes only under real enforcement.
        let target =
            std::env::temp_dir().join(format!("landlock-denied-{}.tmp", std::process::id()));
        let _ = std::fs::remove_file(&target);
        let tool = ToolDefinition::read_only(
            "test.touch-deny",
            "Probe write outside profile",
            "/usr/bin/touch",
            vec![],
            5000,
        );

        let res = execute_tool(&tool, &[target.to_string_lossy().to_string()], None).await;
        assert!(res.is_err(), "write outside profile unexpectedly allowed");
        assert!(!target.exists(), "denied write left a file behind");
        let _ = std::fs::remove_file(&target);
    }

    #[tokio::test]
    async fn test_landlock_honors_write_paths() {
        // A declared write path must stay writable under enforcement.
        let dir = std::env::temp_dir().join(format!("landlock-allowed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("probe.tmp");
        let tool = ToolDefinition::remediate(
            "test.touch-allow",
            "Probe write inside declared path",
            "/usr/bin/touch",
            vec![],
            5000,
            vec![dir.clone()],
        );

        let res = execute_tool(&tool, &[target.to_string_lossy().to_string()], None).await;
        assert!(res.is_ok(), "declared write path denied: {res:?}");
        assert!(target.exists(), "allowed write left no file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_unit_status_with_allowed_exit_code_succeeds() {
        if !std::path::Path::new("/usr/bin/systemctl").exists() {
            return;
        }
        let registry = toold_core::policy::ToolRegistry::with_defaults();
        let tool = registry.get("unit.status").unwrap();
        let res = execute_tool(&tool, &["syntrop-nonexistent-probe.service".into()], None).await;
        assert!(
            res.is_ok(),
            "unit.status for inactive unit should succeed: {:?}",
            res.err()
        );
        let outcome = res.unwrap();
        assert!(tool.allowed_exit_codes.contains(&outcome.exit_code));
        assert!(outcome.exit_code == 3 || outcome.exit_code == 4 || outcome.exit_code == 0);
    }

    #[tokio::test]
    async fn test_net_listeners_reads_listeners() {
        if !std::path::Path::new("/usr/bin/ss").exists() {
            return;
        }
        let registry = toold_core::policy::ToolRegistry::with_defaults();
        let tool = registry.get("net.listeners").unwrap();
        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_ok(), "net.listeners failed: {:?}", res.err());
        let outcome = res.unwrap();
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stdout.contains("Local Address:Port") || outcome.stdout.contains("Netid"));
    }

    #[tokio::test]
    async fn test_zero_shell_policy_rejects_shell_execution() {
        let shell_tool = ToolDefinition::read_only(
            "test.sh",
            "Shell invocation",
            "/bin/sh",
            vec!["-c".into()],
            2000,
        );
        let res = execute_tool(&shell_tool, &["echo pwned".into()], None).await;
        assert!(matches!(res, Err(TooldError::PermissionDenied(_))));

        let bash_tool = ToolDefinition::read_only(
            "test.bash",
            "Bash invocation",
            "/bin/bash",
            vec!["-c".into()],
            2000,
        );
        let res_bash = execute_tool(&bash_tool, &["echo pwned".into()], None).await;
        assert!(matches!(res_bash, Err(TooldError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn test_allowed_exit_code_3_succeeds_and_unallowed_fails() {
        let py_path = "/usr/bin/python3";
        if !std::path::Path::new(py_path).exists() {
            return;
        }
        let mut tool = ToolDefinition::read_only(
            "test.exit3",
            "Exit 3 test",
            py_path,
            vec!["-c".into(), "import sys; sys.exit(3)".into()],
            2000,
        );
        tool.allowed_exit_codes = vec![0, 3, 4];
        let res = execute_tool(&tool, &[], None).await;
        assert!(res.is_ok(), "allowed exit code 3 failed: {:?}", res.err());
        assert_eq!(res.unwrap().exit_code, 3);

        let mut strict_tool = tool.clone();
        strict_tool.allowed_exit_codes = vec![0];
        let strict_res = execute_tool(&strict_tool, &[], None).await;
        assert!(
            matches!(
                strict_res,
                Err(TooldError::ExecutionFailed { exit_code: 3, .. })
            ),
            "unallowed exit code 3 should fail: {:?}",
            strict_res
        );
    }
}

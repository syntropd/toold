//! Unit QA tests for systemd socket activation parsing.

#[cfg(test)]
mod tests {
    use std::env;
    use toold_daemon::activation::{parse_listen_fds, SD_LISTEN_FDS_START};

    /// Restores LISTEN_* on drop so parallel tests never observe our edits.
    struct EnvGuard {
        pid: Option<String>,
        fds: Option<String>,
    }

    impl EnvGuard {
        fn take() -> Self {
            Self {
                pid: env::var("LISTEN_PID").ok(),
                fds: env::var("LISTEN_FDS").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            restore("LISTEN_PID", self.pid.take());
            restore("LISTEN_FDS", self.fds.take());
        }
    }

    fn restore(key: &str, value: Option<String>) {
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn test_listen_fds_negative_paths_yield_no_sockets() {
        // One test holds the process-global env for every sub-case below;
        // the positive path (adopting fd 3) cannot be simulated safely.
        let _guard = EnvGuard::take();

        env::remove_var("LISTEN_PID");
        env::remove_var("LISTEN_FDS");
        assert!(parse_listen_fds().varlink_listener.is_none());

        let wrong_pid = std::process::id().wrapping_add(1).to_string();
        env::set_var("LISTEN_PID", wrong_pid);
        env::set_var("LISTEN_FDS", "1");
        assert!(parse_listen_fds().varlink_listener.is_none());

        env::set_var("LISTEN_PID", "not-a-pid");
        assert!(parse_listen_fds().varlink_listener.is_none());

        // Matching PID with zero FDs warns and adopts nothing (safe:
        // adoption only happens for count >= 1, which we never simulate).
        env::set_var("LISTEN_PID", std::process::id().to_string());
        env::set_var("LISTEN_FDS", "0");
        assert!(parse_listen_fds().varlink_listener.is_none());
    }

    #[test]
    fn test_sd_listen_fds_start_is_fd3() {
        assert_eq!(SD_LISTEN_FDS_START, 3);
    }
}

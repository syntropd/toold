//! Systemd socket activation implementation for toold.

use std::env;
use std::os::unix::io::{FromRawFd, RawFd};
use tokio::net::UnixListener;
use tracing::warn;

/// The starting file descriptor index passed by systemd (FD 3).
pub const SD_LISTEN_FDS_START: RawFd = 3;

/// Adopted systemd sockets for toold.
#[derive(Debug)]
pub struct ActivatedSockets {
    /// Primary Varlink IPC Unix listener (FD 3).
    pub varlink_listener: Option<UnixListener>,
}

/// Parses systemd environment variables and adopts pre-bound Unix sockets.
pub fn parse_listen_fds() -> ActivatedSockets {
    let pid_matches = match env::var("LISTEN_PID") {
        Ok(pid_str) => pid_str.parse::<u32>().map(|p| p == std::process::id()).unwrap_or(false),
        Err(_) => false,
    };

    if !pid_matches {
        return ActivatedSockets { varlink_listener: None };
    }

    let count: usize = env::var("LISTEN_FDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let varlink_listener = if count >= 1 {
        adopt_unix_listener(SD_LISTEN_FDS_START)
    } else {
        warn!(
            "systemd socket activation indicated LISTEN_FDS={} but expected at least 1; varlink listener will not be attached",
            count
        );
        None
    };

    ActivatedSockets { varlink_listener }
}

fn adopt_unix_listener(fd: RawFd) -> Option<UnixListener> {
    unsafe {
        let std_listener = std::os::unix::net::UnixListener::from_raw_fd(fd);
        if let Err(e) = std_listener.set_nonblocking(true) {
            warn!(
                "Failed to set nonblocking on adopted fd {}: {}; listener will fall back to blocking mode",
                fd, e
            );
        }
        match UnixListener::from_std(std_listener) {
            Ok(listener) => Some(listener),
            Err(e) => {
                warn!(
                    "Failed to convert adopted fd {} into a tokio UnixListener: {}; the FD may be the wrong type",
                    fd, e
                );
                None
            }
        }
    }
}

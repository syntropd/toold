//! Pure Rust systemd sd_notify implementation for toold.

use std::env;
use std::os::fd::AsFd;
use std::os::unix::net::UnixDatagram;
use std::path::Path;

/// Maximum notify datagram size (systemd protocol cap).
const NOTIFY_MAX: usize = 8 * 1024 * 1024;

/// Build a `SocketAddrUnix` for the notify socket, supporting both
/// filesystem paths (`/run/systemd/notify`) and abstract namespaces
/// (`@notify`). Abstract names containing interior NULs are rejected
/// because the kernel would route to an address nobody listens on,
/// silently dropping the notification.
fn notify_address(socket_path: &str) -> Option<rustix::net::SocketAddrUnix> {
    if let Some(name) = socket_path.strip_prefix('@') {
        if name.is_empty() || name.as_bytes().contains(&0) {
            return None;
        }
        rustix::net::SocketAddrUnix::new_abstract_name(name.as_bytes()).ok()
    } else if socket_path.is_empty() {
        None
    } else {
        rustix::net::SocketAddrUnix::new(Path::new(socket_path)).ok()
    }
}

/// Strip embedded newlines so a caller-supplied status string cannot
/// terminate the current sd_notify variable early and inject a fake
/// following variable.
fn sanitize_value(s: &str) -> String {
    s.chars().filter(|&c| c != '\n' && c != '\r').collect()
}

/// Sends a formatted raw notification string to `$NOTIFY_SOCKET`.
///
/// Returns `false` for any failure (missing env, oversized payload,
/// invalid address, sendto error). The caller is expected to treat a
/// `false` return as "systemd didn't get the notification" — there is
/// no error code because the sd_notify protocol is fire-and-forget.
pub fn send_notify(state: &str) -> bool {
    if state.len() > NOTIFY_MAX {
        return false;
    }
    let socket_path = match env::var("NOTIFY_SOCKET") {
        Ok(path) if !path.is_empty() => path,
        _ => return false,
    };

    let address = match notify_address(&socket_path) {
        Some(addr) => addr,
        None => return false,
    };

    let socket = match UnixDatagram::unbound() {
        Ok(s) => s,
        Err(_) => return false,
    };

    rustix::net::sendto_unix(
        socket.as_fd(),
        state.as_bytes(),
        rustix::net::SendFlags::empty(),
        &address,
    )
    .is_ok()
}

/// Emits the READY=1 readiness notification.
pub fn notify_ready() -> bool {
    send_notify("READY=1\n")
}

/// Emits an updated STATUS string. Embedded newlines and carriage
/// returns are stripped so the variable cannot be terminated early.
pub fn notify_status(status: &str) -> bool {
    send_notify(&format!("STATUS={}\n", sanitize_value(status)))
}

/// Emits the WATCHDOG=1 heartbeat ping.
pub fn notify_watchdog() -> bool {
    send_notify("WATCHDOG=1\n")
}

/// Emits the STOPPING=1 shutdown signal.
pub fn notify_stopping() -> bool {
    send_notify("STOPPING=1\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_address_filesystem_path() {
        let addr = notify_address("/run/systemd/notify").unwrap();
        assert!(addr.path().is_some());
    }

    #[test]
    fn test_notify_address_abstract_namespace() {
        let addr = notify_address("@notify").unwrap();
        assert_eq!(addr.abstract_name().unwrap(), b"notify");
    }

    #[test]
    fn test_notify_address_empty_returns_none() {
        assert!(notify_address("").is_none());
        assert!(notify_address("@").is_none());
    }

    #[test]
    fn test_notify_address_abstract_with_interior_nul_fails() {
        assert!(notify_address("@notify\0anything").is_none());
    }

    #[test]
    fn test_notify_status_sanitizes_newlines() {
        let sanitized = sanitize_value("evil\nMAINPID=42\n");
        assert_eq!(sanitized, "evilMAINPID=42");
        assert!(!sanitized.contains('\n'));
    }

    #[test]
    fn test_notify_oversize_message_rejected() {
        let huge = "X".repeat(NOTIFY_MAX + 1);
        assert!(!send_notify(&huge));
    }
}

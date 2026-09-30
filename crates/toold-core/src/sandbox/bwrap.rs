//! Bubblewrap unprivileged container sandbox builder with Landlock fallback.

use crate::policy::rule::ToolDefinition;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Check if `bwrap` binary is available on the system.
pub fn find_bwrap_binary() -> Option<PathBuf> {
    for candidate in &["/usr/bin/bwrap", "/bin/bwrap", "/usr/local/bin/bwrap"] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Check if `bwrap` can actually execute in the current host environment
/// (e.g. unprivileged user namespaces and mount propagation are permitted).
pub fn is_bwrap_supported() -> bool {
    static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        let Some(bwrap) = find_bwrap_binary() else {
            return false;
        };
        std::process::Command::new(bwrap)
            .args(["--ro-bind", "/", "/", "true"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Construct a sandboxed Bubblewrap command if `bwrap` is installed and operational.
/// Applies namespace isolation, system read-only mounts, and isolated tmpfs.
pub fn build_bwrap_command(
    tool: &ToolDefinition,
    user_args: &[String],
    working_dir: Option<&Path>,
) -> Option<Command> {
    if !is_bwrap_supported() {
        return None;
    }
    let bwrap = find_bwrap_binary()?;
    let mut cmd = Command::new(bwrap);

    // Namespace isolation and lifecycle containment
    cmd.arg("--unshare-user");
    cmd.arg("--unshare-ipc");
    cmd.arg("--unshare-pid");
    cmd.arg("--unshare-uts");
    cmd.arg("--unshare-cgroup");
    if !tool.requires_network_access {
        cmd.arg("--unshare-net");
    }
    cmd.arg("--die-with-parent");

    // Standard root read-only system binds
    for dir in &["/usr", "/bin", "/sbin", "/lib", "/lib64"] {
        if Path::new(dir).exists() {
            cmd.arg("--ro-bind").arg(dir).arg(dir);
        }
    }
    if Path::new("/etc").exists() {
        cmd.arg("--ro-bind").arg("/etc").arg("/etc");
    }
    if Path::new("/dev").exists() {
        cmd.arg("--dev").arg("/dev");
    }
    if Path::new("/proc").exists() {
        cmd.arg("--proc").arg("/proc");
    }

    // Ephemeral scratch isolation
    cmd.arg("--tmpfs").arg("/tmp");
    cmd.arg("--tmpfs").arg("/run");

    if tool.requires_systemd_socket {
        for sock in &["/run/dbus/system_bus_socket", "/run/systemd/private"] {
            if Path::new(sock).exists() {
                cmd.arg("--ro-bind").arg(sock).arg(sock);
            }
        }
        if Path::new("/run/systemd/system").exists() {
            let has_write = tool
                .write_paths
                .iter()
                .any(|p| p.starts_with("/run/systemd/system"));
            if has_write {
                cmd.arg("--bind")
                    .arg("/run/systemd/system")
                    .arg("/run/systemd/system");
            } else {
                cmd.arg("--ro-bind")
                    .arg("/run/systemd/system")
                    .arg("/run/systemd/system");
            }
        }
    }

    // Whitelisted path binds from tool policy
    for ro in &tool.read_paths {
        if ro.exists() {
            cmd.arg("--ro-bind").arg(ro).arg(ro);
        }
    }
    let has_tmp_write = tool.write_paths.iter().any(|p| p.starts_with("/tmp"));
    for rw in &tool.write_paths {
        if rw.exists() && (!tool.requires_systemd_socket || !rw.starts_with("/run/systemd/system"))
        {
            cmd.arg("--bind").arg(rw).arg(rw);
        }
    }
    if !has_tmp_write {
        cmd.arg("--remount-ro").arg("/tmp");
    }

    let cwd = working_dir.unwrap_or(Path::new("/run"));
    cmd.arg("--chdir").arg(cwd);

    // Target executable boundary
    cmd.arg("--").arg(&tool.binary_path);
    for arg in &tool.fixed_args {
        cmd.arg(arg);
    }
    for arg in user_args {
        cmd.arg(arg);
    }

    Some(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_bwrap_or_none() {
        let bwrap = find_bwrap_binary();
        if let Some(path) = bwrap {
            assert!(path.exists());
        }
    }
}

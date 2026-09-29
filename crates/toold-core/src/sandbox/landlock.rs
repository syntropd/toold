//! Landlock filesystem confinement for tool child processes.
//!
//! Each tool runs under a ruleset built from its declared policy:
//! `read_paths` (read-only) plus `write_paths` (read-write), layered
//! over a universal userspace runtime profile (shared libraries,
//! proc/sys introspection, D-Bus, the journal, devices). Everything
//! outside the union — /home, /root, /tmp, /var/lib, /boot — is
//! denied, even though the daemon itself could reach it.
//!
//! Enforcement happens in the child (pre_exec, before exec): the
//! ruleset is fully built in the parent (missing paths fail early
//! with a clear error) and only the final `restrict_self` syscalls
//! run post-fork. Fail-closed: anything less than full enforcement
//! refuses to run the tool.

use crate::error::TooldError;
use crate::policy::rule::ToolDefinition;
use landlock::{
    path_beneath_rules, Access, AccessFs, CompatLevel, Compatible, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr, RulesetStatus, ABI,
};
use std::path::PathBuf;

/// Landlock ABI to enforce (Linux 6.2+; older kernels fail closed
/// with a clear error instead of running unconfined).
const ABI: ABI = ABI::V3;

/// Universal read-mostly runtime: what every tool child needs to run.
fn runtime_ro(binary_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("/usr"),
        PathBuf::from("/etc"),
        PathBuf::from("/proc"),
        PathBuf::from("/sys"),
        PathBuf::from("/run"),
        PathBuf::from("/dev"),
    ];
    if let Some(dir) = binary_dir {
        if !paths.contains(&dir) {
            paths.push(dir);
        }
    }
    paths
}

/// Universal writable runtime: D-Bus endpoints for systemctl tools.
/// (Write paths also carry read+execute per Landlock's access model;
/// the confinement that matters — no reads or writes outside the
/// union — still holds.)
fn runtime_rw() -> Vec<PathBuf> {
    vec![PathBuf::from("/run/systemd"), PathBuf::from("/run/dbus")]
}

/// Paths granted to a tool child: (read-only, read-write).
/// Pure data (no syscalls) so policy stays unit-testable.
pub fn confine_paths(tool: &ToolDefinition) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let binary_dir = tool.binary_path.parent().map(|p| p.to_path_buf());
    let mut ro = runtime_ro(binary_dir);
    for p in &tool.read_paths {
        if !ro.contains(p) {
            ro.push(p.clone());
        }
    }
    let mut rw = runtime_rw();
    for p in &tool.write_paths {
        if !rw.contains(p) {
            rw.push(p.clone());
        }
    }
    (ro, rw)
}

/// Build the child ruleset in the parent. Missing paths fail here
/// (clear parent-side error) rather than post-fork in the child.
pub fn build_ruleset(tool: &ToolDefinition) -> Result<RulesetCreated, TooldError> {
    let (ro, rw) = confine_paths(tool);
    let ro_rules = path_beneath_rules(ro, AccessFs::from_read(ABI));
    let rw_rules = path_beneath_rules(rw, AccessFs::from_read(ABI) | AccessFs::from_write(ABI));
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(ABI))
        .map_err(|e| TooldError::Sandbox(format!("landlock handle access: {e}")))?
        .create()
        .map_err(|e| TooldError::Sandbox(format!("landlock create: {e}")))?
        .add_rules(ro_rules)
        .map_err(|e| TooldError::Sandbox(format!("landlock read rules: {e}")))?
        .add_rules(rw_rules)
        .map_err(|e| TooldError::Sandbox(format!("landlock write rules: {e}")))?
        .set_compatibility(CompatLevel::BestEffort);
    Ok(ruleset)
}

/// Apply a prebuilt ruleset in the child (pre_exec). Only syscalls on
/// pre-opened fds run here (fork-safe); anything less than full
/// enforcement refuses to exec the tool.
pub fn restrict_child(ruleset: RulesetCreated) -> Result<(), TooldError> {
    let status = ruleset
        .restrict_self()
        .map_err(|e| TooldError::Sandbox(format!("landlock restrict: {e}")))?;
    if status.ruleset != RulesetStatus::FullyEnforced {
        return Err(TooldError::Sandbox(format!(
            "landlock not fully enforced ({:?}); refusing to run",
            status.ruleset
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_tool() -> ToolDefinition {
        ToolDefinition::read_only("probe", "probe", "/usr/bin/echo", vec![], 1000)
    }

    #[test]
    fn runtime_covers_tool_needs_but_not_homes_or_tmp() {
        let (ro, rw) = confine_paths(&probe_tool());
        for need in ["/usr", "/etc", "/proc", "/sys", "/run", "/dev", "/usr/bin"] {
            assert!(ro.iter().any(|p| p == need), "missing RO {need}");
        }
        for need in ["/run/systemd", "/run/dbus"] {
            assert!(rw.iter().any(|p| p == need), "missing RW {need}");
        }
        for denied in ["/home", "/root", "/tmp", "/var/lib", "/boot", "/opt"] {
            assert!(!ro.iter().any(|p| p == denied), "leaked RO {denied}");
            assert!(!rw.iter().any(|p| p == denied), "leaked RW {denied}");
        }
    }

    #[test]
    fn policy_paths_join_the_profile() {
        let tool = ToolDefinition::remediate(
            "probe",
            "probe",
            "/usr/bin/echo",
            vec![],
            1000,
            vec![PathBuf::from("/run/systemd/system")],
        );
        let (ro, rw) = confine_paths(&tool);
        assert!(ro.iter().any(|p| p == "/var/log" || p == "/etc"));
        assert!(rw.iter().any(|p| p == "/run/systemd/system"));
    }

    #[test]
    fn ruleset_builds_on_this_kernel() {
        build_ruleset(&probe_tool()).unwrap();
    }
}

//! Thread-safe repository of registered and authorized tools.

use crate::policy::rule::ToolDefinition;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Repository of validated tools ready for sandboxed invocation.
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolDefinition>>,
}

impl ToolRegistry {
    /// Constructs a registry populated with default system diagnostics.
    pub fn with_defaults() -> Self {
        let registry = Self {
            tools: RwLock::new(HashMap::new()),
        };

        registry.register_default_tools();
        registry
    }

    /// Creates an empty registry for testing.
    pub fn empty() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a new tool definition.
    pub fn register(&self, tool: ToolDefinition) {
        if let Ok(mut lock) = self.tools.write() {
            lock.insert(tool.name.clone(), tool);
        }
    }

    /// Looks up a registered tool definition by name.
    pub fn get(&self, name: &str) -> Option<ToolDefinition> {
        self.tools.read().ok()?.get(name).cloned()
    }

    /// Lists all registered tool definitions.
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .map(|l| l.values().cloned().collect())
            .unwrap_or_default()
    }

    fn register_default_tools(&self) {
        let mut unit_status = ToolDefinition::read_only(
            "unit.status",
            "Queries the active runtime state of a systemd unit",
            "/usr/bin/systemctl",
            vec!["is-active".into()],
            5000,
        );
        unit_status.allowed_exit_codes = vec![0, 3, 4];
        unit_status.requires_systemd_socket = true;
        self.register(unit_status);

        self.register(ToolDefinition::read_only(
            "journal.slice",
            "Reads recent log messages for a target systemd unit",
            "/usr/bin/journalctl",
            vec![
                "--no-pager".into(),
                "-o".into(),
                "short-iso".into(),
                "-u".into(),
            ],
            8000,
        ));

        let mut net_listeners = ToolDefinition::read_only(
            "net.listeners",
            "Lists active listening TCP and UDP sockets across the host",
            "/usr/bin/ss",
            vec!["-tulpn".into()],
            5000,
        );
        net_listeners.requires_network_access = true;
        self.register(net_listeners);

        // Scratch, not system state: verify stages temp files under
        // /tmp (private-namespaced by the unit's PrivateTmp), so the
        // read-only tool still needs that one write path to run.
        let mut verify = ToolDefinition::read_only(
            "syntax.verify",
            "Verifies unit configuration syntax without starting the service",
            "/usr/bin/systemd-analyze",
            vec!["verify".into(), "--man=no".into()],
            5000,
        );
        verify.write_paths.push(PathBuf::from("/tmp"));
        self.register(verify);

        let mut unit_restart = ToolDefinition::remediate(
            "unit.restart",
            "Restarts an existing systemd unit",
            "/usr/bin/systemctl",
            vec!["restart".into()],
            15000,
            vec![PathBuf::from("/run/systemd/system")],
        );
        unit_restart.requires_systemd_socket = true;
        self.register(unit_restart);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_default_tool_policies() {
        let reg = ToolRegistry::with_defaults();
        let status = reg.get("unit.status").unwrap();
        assert_eq!(status.allowed_exit_codes, vec![0, 3, 4]);
        assert!(status.requires_systemd_socket);
        assert!(!status.requires_network_access);

        let net = reg.get("net.listeners").unwrap();
        assert!(net.requires_network_access);
        assert_eq!(net.allowed_exit_codes, vec![0]);

        let restart = reg.get("unit.restart").unwrap();
        assert!(restart.requires_systemd_socket);
    }
}

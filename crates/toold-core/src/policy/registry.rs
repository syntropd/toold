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
        self.register(ToolDefinition::read_only(
            "unit.status",
            "Queries the active runtime state of a systemd unit",
            "/usr/bin/systemctl",
            vec!["is-active".into()],
            5000,
        ));

        self.register(ToolDefinition::read_only(
            "journal.slice",
            "Reads recent log messages for a target systemd unit",
            "/usr/bin/journalctl",
            vec!["--no-pager".into(), "-o".into(), "short-iso".into()],
            8000,
        ));

        self.register(ToolDefinition::read_only(
            "net.listeners",
            "Lists active listening TCP and UDP sockets across the host",
            "/usr/bin/ss",
            vec!["-tulpn".into()],
            5000,
        ));

        self.register(ToolDefinition::read_only(
            "syntax.verify",
            "Verifies unit configuration syntax without starting the service",
            "/usr/bin/systemd-analyze",
            vec!["verify".into()],
            5000,
        ));

        self.register(ToolDefinition::remediate(
            "unit.restart",
            "Restarts an existing systemd unit",
            "/usr/bin/systemctl",
            vec!["restart".into()],
            15000,
            vec![PathBuf::from("/run/systemd/system")],
        ));
    }
}

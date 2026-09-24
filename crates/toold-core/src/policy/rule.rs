//! Security policy rules and execution mode definitions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Execution privileges allowed for a given tool primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Read-only diagnostic tool. Cannot modify filesystem or system state.
    ReadOnly,
    /// Remediating action with mandatory pre-execution rollback snapshot.
    RemediateWithRollback,
    /// Guarded action requiring explicit administrative authorization.
    Guarded,
}

/// Declarative security specification for an executable tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Canonical tool name (e.g. "unit.status", "journal.slice").
    pub name: String,
    /// Human-readable purpose description in ASD-STE100 technical English.
    pub description: String,
    /// Safety privilege classification.
    pub mode: ExecutionMode,
    /// Absolute path to executable binary on the host filesystem.
    pub binary_path: PathBuf,
    /// Fixed immutable arguments prepended to every invocation.
    pub fixed_args: Vec<String>,
    /// Maximum allowed execution duration in milliseconds before SIGKILL.
    pub timeout_ms: u64,
    /// Permitted readable host filesystem paths under sandbox.
    pub read_paths: Vec<PathBuf>,
    /// Permitted writable host filesystem paths under sandbox.
    pub write_paths: Vec<PathBuf>,
}

impl ToolDefinition {
    /// Creates a new read-only diagnostic tool definition.
    pub fn read_only<P: Into<PathBuf>>(
        name: &str,
        description: &str,
        binary_path: P,
        fixed_args: Vec<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            mode: ExecutionMode::ReadOnly,
            binary_path: binary_path.into(),
            fixed_args,
            timeout_ms,
            read_paths: vec![PathBuf::from("/etc"), PathBuf::from("/var/log")],
            write_paths: Vec::new(),
        }
    }

    /// Creates a remediating tool definition with rollback requirement.
    pub fn remediate<P: Into<PathBuf>>(
        name: &str,
        description: &str,
        binary_path: P,
        fixed_args: Vec<String>,
        timeout_ms: u64,
        write_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            mode: ExecutionMode::RemediateWithRollback,
            binary_path: binary_path.into(),
            fixed_args,
            timeout_ms,
            read_paths: vec![PathBuf::from("/etc"), PathBuf::from("/run")],
            write_paths,
        }
    }
}

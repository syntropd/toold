//! Configuration parser and defaults for toold.

use crate::error::TooldError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default configuration file location for toold.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/syntrop/toold.conf";

/// Default runtime socket path for toold Varlink interface.
pub const DEFAULT_SOCKET_PATH: &str = "/run/syntrop/io.syntrop.Tool1";

/// Default storage directory for rollback snapshots and execution logs.
pub const DEFAULT_STORAGE_PATH: &str = "/var/lib/toold";

/// Daemon operational configuration parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TooldConfig {
    /// Directory for rollback journal storage.
    pub storage_path: PathBuf,
    /// Path to Varlink IPC Unix domain socket.
    pub socket_path: PathBuf,
    /// Default tool execution timeout in milliseconds.
    pub default_timeout_ms: u64,
    /// Maximum stdout/stderr output bytes retained per execution.
    pub max_output_bytes: usize,
}

impl Default for TooldConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from(DEFAULT_STORAGE_PATH),
            socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
            default_timeout_ms: 10000,
            max_output_bytes: 65536,
        }
    }
}

impl TooldConfig {
    /// Loads configuration from a filesystem path, falling back to defaults if not found.
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Result<Self, TooldError> {
        let p = path.as_ref();
        if !p.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(p)?;
        toml::from_str(&content).map_err(|e| TooldError::Config(e.to_string()))
    }
}

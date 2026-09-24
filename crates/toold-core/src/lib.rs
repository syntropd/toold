//! Core engine for toold: Sandboxed Action and Diagnostic Execution.
//!
//! Provides declarative tool policies, sandboxed child process execution,
//! and rollback journaling for safe autonomous remediations.

pub mod config;
pub mod error;
pub mod journal;
pub mod policy;
pub mod sandbox;

pub use config::{TooldConfig, DEFAULT_CONFIG_PATH, DEFAULT_SOCKET_PATH, DEFAULT_STORAGE_PATH};
pub use error::TooldError;
pub use journal::{RollbackJournal, RollbackRecord};
pub use policy::{ExecutionMode, ToolDefinition, ToolRegistry};
pub use sandbox::{execute_tool, ExecutionResult};

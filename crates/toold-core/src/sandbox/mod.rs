//! Sandboxed process execution and runtime confinement.

pub mod runner;

pub use runner::{execute_tool, ExecutionResult};

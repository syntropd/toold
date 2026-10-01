//! Sandboxed process execution and runtime confinement.

pub mod bwrap;
pub mod code_loop;
pub mod diagnostic;
pub mod landlock;
pub mod runner;

pub use bwrap::build_bwrap_command;
pub use code_loop::{run_self_correction_loop, CodeLoopResult, ModelCompleter, MAX_ITERATIONS};
pub use diagnostic::{parse_diagnostics, Diagnostic};
pub use runner::{execute_tool, ExecutionResult};

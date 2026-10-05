//! Daemon service implementation for toold: Sandboxed Action and Diagnostic Execution.
//!
//! Exposes activation descriptor loading, systemd watchdog notifications,
//! asynchronous tool execution runners, autonomous self-correction loops,
//! and the `io.syntrop.Tool1` Varlink interface.

pub mod activation;
pub mod notify;
pub mod runner;
pub mod varlink;

pub use activation::{parse_listen_fds, ActivatedSockets};
pub use notify::{notify_ready, notify_status, notify_stopping, notify_watchdog, send_notify};
pub use runner::CodeLoopResult;
pub use runner::ExecutionResult;
pub use runner::HttpCompleter;
pub use runner::ModelCompleter;
pub use runner::VarlinkCompleter;
pub use runner::execute_tool;
pub use runner::run_self_correction_loop;
pub use varlink::{Tool1Handler, VarlinkCall, VarlinkReply, VarlinkServer};

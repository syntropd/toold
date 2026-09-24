//! Subcommand handlers for toolctl CLI.

pub mod completions_cmd;
pub mod history_cmd;
pub mod info_cmd;
pub mod list_cmd;
pub mod rollback_cmd;
pub mod run_cmd;

pub use completions_cmd::exec_completions;
pub use history_cmd::exec_history;
pub use info_cmd::exec_info;
pub use list_cmd::exec_list;
pub use rollback_cmd::exec_rollback;
pub use run_cmd::exec_run;

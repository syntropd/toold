//! CLI argument definitions and command structures for toolctl.

use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;
use toold_core::config::DEFAULT_SOCKET_PATH;

/// CLI client for toold sandboxed action and diagnostic execution daemon.
#[derive(Parser, Debug)]
#[command(
    name = "toolctl",
    version,
    about = "Control and invoke sandboxed system actions via toold",
    long_about = "Execute diagnostic and remediating system tools within bounded sandboxes."
)]
pub struct Cli {
    /// Path to toold Varlink Unix domain socket.
    #[arg(short = 's', long = "socket", default_value = DEFAULT_SOCKET_PATH, global = true)]
    pub socket: PathBuf,

    /// Output results in formatted JSON.
    #[arg(long = "json", global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for toolctl.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all registered and permitted diagnostic/remediating tools.
    List,

    /// Execute a tool within the toold sandbox.
    Run {
        /// Name of registered tool to execute (e.g. unit.status, journal.slice).
        tool: String,

        /// Associated target systemd unit, if applicable.
        #[arg(short = 'u', long = "unit")]
        target_unit: Option<String>,

        /// Trailing arguments passed directly to the tool.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Roll back a prior remediating action using its recorded snapshot.
    Rollback {
        /// Unique snapshot identifier (e.g. rb-1790235247-1).
        rollback_id: String,
    },

    /// List historical rollback snapshots.
    History {
        /// Time window in seconds to inspect.
        #[arg(short = 'w', long = "since", default_value = "86400")]
        since: u64,

        /// Maximum records to display.
        #[arg(short = 'l', long = "limit", default_value = "50")]
        limit: usize,
    },

    /// Inspect daemon vendor information and interface schemas.
    Info,

    /// Generate shell auto-completion script.
    Completions {
        /// Target shell for completion generation.
        #[arg(value_enum)]
        shell: Shell,
    },
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_with_defaults() {
        let cli = Cli::try_parse_from(["toolctl", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::List));
        assert!(!cli.json);
        assert_eq!(cli.socket, PathBuf::from(DEFAULT_SOCKET_PATH));
    }

    #[test]
    fn test_parse_global_flags() {
        let cli = Cli::try_parse_from(["toolctl", "-s", "/tmp/x.sock", "--json", "info"]).unwrap();
        assert!(cli.json);
        assert_eq!(cli.socket, PathBuf::from("/tmp/x.sock"));
        assert!(matches!(cli.command, Commands::Info));
    }

    #[test]
    fn test_parse_run_with_unit_and_args() {
        let cli = Cli::try_parse_from([
            "toolctl",
            "run",
            "unit.status",
            "-u",
            "sshd.service",
            "--",
            "extra",
        ])
        .unwrap();
        match cli.command {
            Commands::Run {
                tool,
                target_unit,
                args,
            } => {
                assert_eq!(tool, "unit.status");
                assert_eq!(target_unit.as_deref(), Some("sshd.service"));
                assert_eq!(args, vec!["extra".to_string()]);
            }
            _ => panic!("expected run subcommand"),
        }
    }

    #[test]
    fn test_parse_rollback() {
        let cli = Cli::try_parse_from(["toolctl", "rollback", "rb-1790235247-1"]).unwrap();
        match cli.command {
            Commands::Rollback { rollback_id } => assert_eq!(rollback_id, "rb-1790235247-1"),
            _ => panic!("expected rollback subcommand"),
        }
    }

    #[test]
    fn test_parse_history_defaults_and_overrides() {
        let cli = Cli::try_parse_from(["toolctl", "history"]).unwrap();
        match cli.command {
            Commands::History { since, limit } => {
                assert_eq!(since, 86400);
                assert_eq!(limit, 50);
            }
            _ => panic!("expected history subcommand"),
        }
        let cli = Cli::try_parse_from(["toolctl", "history", "-w", "60", "-l", "5"]).unwrap();
        match cli.command {
            Commands::History { since, limit } => {
                assert_eq!(since, 60);
                assert_eq!(limit, 5);
            }
            _ => panic!("expected history subcommand"),
        }
    }

    #[test]
    fn test_parse_completions() {
        let cli = Cli::try_parse_from(["toolctl", "completions", "bash"]).unwrap();
        match cli.command {
            Commands::Completions { shell } => assert!(matches!(shell, Shell::Bash)),
            _ => panic!("expected completions subcommand"),
        }
    }

    #[test]
    fn test_parse_unknown_subcommand_fails() {
        assert!(Cli::try_parse_from(["toolctl", "bogus"]).is_err());
        assert!(Cli::try_parse_from(["toolctl", "run"]).is_err());
    }
}

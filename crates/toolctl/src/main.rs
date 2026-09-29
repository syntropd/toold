//! Main binary entry point for toolctl CLI.

use anyhow::Result;
use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Shell};
use std::io;
use toolctl::cli::{Cli, Commands};
use toolctl::client::TooldClient;
use toolctl::cmd::*;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = TooldClient::new(&cli.socket);

    match cli.command {
        Commands::List => {
            exec_list(&client, cli.json).await?;
        }
        Commands::Run {
            tool,
            target_unit,
            args,
        } => {
            exec_run(&client, &tool, &args, target_unit.as_deref(), cli.json).await?;
        }
        Commands::Rollback { rollback_id } => {
            exec_rollback(&client, &rollback_id, cli.json).await?;
        }
        Commands::History { since, limit } => {
            exec_history(&client, since, limit, cli.json).await?;
        }
        Commands::Info => {
            exec_info(&client, cli.json).await?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            exec_completions(&mut cmd, shell);
        }
    }

    Ok(())
}

/// Generates shell completion script to stdout.
fn exec_completions(cmd: &mut Command, shell: Shell) {
    let bin_name = cmd.get_name().to_string();
    generate(shell, cmd, bin_name, &mut io::stdout());
}

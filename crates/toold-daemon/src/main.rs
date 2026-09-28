//! Entry point for toold: Sandboxed Action and Diagnostic Execution Daemon.

use anyhow::{Context, Result};
use std::fs;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};
use toold_core::config::{TooldConfig, DEFAULT_CONFIG_PATH};
use toold_core::journal::RollbackJournal;
use toold_core::policy::ToolRegistry;
use toold_daemon::activation::parse_listen_fds;
use toold_daemon::notify::{notify_ready, notify_stopping};
use toold_daemon::varlink::{Tool1Handler, VarlinkServer};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting toold (Sandboxed Agentic Action & Diagnostic Execution Daemon)");

    let args: Vec<String> = std::env::args().collect();
    let mut config_path =
        std::env::var("TOOLD_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
    if let Some(path) = config_path_from_args(&args) {
        config_path = path;
    }

    let config = TooldConfig::load_or_default(&config_path)
        .context("Failed loading toold configuration")?;

    fs::create_dir_all(&config.storage_path)
        .context("Failed creating storage directory")?;

    let registry = Arc::new(ToolRegistry::with_defaults());
    let journal = Arc::new(RollbackJournal::new(&config.storage_path)?);

    let handler = Tool1Handler::new(Arc::clone(&registry), Arc::clone(&journal));

    let activated = parse_listen_fds();
    let listener = match activated.varlink_listener {
        Some(l) => {
            info!("Adopted socket from systemd socket activation");
            l
        }
        None => {
            let socket_path = &config.socket_path;
            if let Some(parent) = socket_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if socket_path.exists() {
                let _ = fs::remove_file(socket_path);
            }
            info!("Listening on Unix domain socket: {}", socket_path.display());
            let std_listener = StdUnixListener::bind(socket_path)?;
            std_listener.set_nonblocking(true)?;
            UnixListener::from_std(std_listener)?
        }
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let server = VarlinkServer::new(listener, handler, shutdown_rx);

    notify_ready();
    info!("toold successfully initialized and ready");

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        res = server.run() => {
            if let Err(e) = res {
                error!("Varlink server error: {}", e);
            }
        }
        _ = sigterm.recv() => {
            info!("Received SIGTERM, initiating shutdown");
            let _ = shutdown_tx.send(true);
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, initiating shutdown");
            let _ = shutdown_tx.send(true);
        }
    }

    notify_stopping();
    info!("toold shutdown completed");
    Ok(())
}

/// Resolves a `--config`/`-c` path override from raw process arguments.
///
/// Pure scan of argv: returns the value following the first config flag,
/// or `None` when the flag is absent or has no following value.
fn config_path_from_args(args: &[String]) -> Option<String> {
    args.iter()
        .position(|a| a == "--config" || a == "-c")
        .and_then(|pos| args.get(pos + 1).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn test_config_flag_long_and_short() {
        assert_eq!(
            config_path_from_args(&argv(&["toold", "--config", "/tmp/a.toml"])),
            Some("/tmp/a.toml".to_string())
        );
        assert_eq!(
            config_path_from_args(&argv(&["toold", "-c", "/tmp/b.toml"])),
            Some("/tmp/b.toml".to_string())
        );
    }

    #[test]
    fn test_config_flag_absent_or_dangling() {
        assert_eq!(config_path_from_args(&argv(&["toold"])), None);
        assert_eq!(config_path_from_args(&argv(&["toold", "--config"])), None);
        assert_eq!(config_path_from_args(&argv(&["toold", "-c"])), None);
    }

    #[test]
    fn test_config_flag_first_wins() {
        let args = argv(&["toold", "-c", "/tmp/a.toml", "--config", "/tmp/b.toml"]);
        assert_eq!(
            config_path_from_args(&args),
            Some("/tmp/a.toml".to_string())
        );
    }
}

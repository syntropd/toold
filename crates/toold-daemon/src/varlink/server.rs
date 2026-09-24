//! Varlink Unix domain socket listener and protocol dispatcher for toold.

use super::protocol::{VarlinkCall, VarlinkReply};
use super::service::handle_service_call;
use super::tool1::Tool1Handler;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Maximum bytes accepted on a single client connection before it is dropped.
///
/// Caps memory growth so a hostile client that never sends a NUL terminator
/// cannot OOM the daemon. None of the methods declared in
/// `IO_SYNTROP_TOOL1_INTERFACE` legitimately approach this size.
pub const MAX_MSG_BYTES: usize = 1024 * 1024;

/// Varlink server instance listening on a Unix domain socket.
pub struct VarlinkServer {
    listener: UnixListener,
    handler: Arc<Tool1Handler>,
    shutdown: watch::Receiver<bool>,
}

impl VarlinkServer {
    /// Constructs a VarlinkServer from an open UnixListener.
    pub fn new(listener: UnixListener, handler: Tool1Handler) -> Self {
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            listener,
            handler: Arc::new(handler),
            shutdown: shutdown_rx,
        }
    }

    /// Runs the accept and dispatch loop until cancelled or `shutdown` flips.
    pub async fn run(self) -> Result<()> {
        info!("toold Varlink server accepting connections");
        let VarlinkServer {
            listener,
            handler,
            mut shutdown,
        } = self;
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("Varlink listener shutting down on signal");
                        return Ok(());
                    }
                }
                accept = listener.accept() => match accept {
                    Ok((stream, _)) => {
                        let handler = Arc::clone(&handler);
                        let sd = shutdown.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, handler, sd).await {
                                debug!("Client connection closed: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                        return Err(e).context("Failed accepting Varlink stream");
                    }
                }
            }
        }
    }
}

async fn handle_client(
    mut stream: UnixStream,
    handler: Arc<Tool1Handler>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let mut buffer: Vec<u8> = Vec::with_capacity(4096);
    let mut read_chunk = [0u8; 1024];

    loop {
        tokio::select! {
            biased;
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    return Ok(());
                }
            }
            read = stream.read(&mut read_chunk) => {
                let n = match read {
                    Ok(n) => n,
                    Err(e) => return Err(e.into()),
                };
                if n == 0 {
                    break;
                }

                buffer.extend_from_slice(&read_chunk[..n]);

                // Bound the buffer: a hostile client that never sends a NUL
                // must not be able to OOM the daemon.
                if buffer.len() > MAX_MSG_BYTES {
                    warn!(
                        "Varlink client exceeded {} bytes without NUL; closing",
                        MAX_MSG_BYTES
                    );
                    let err_reply = VarlinkReply::err(
                        "org.varlink.service.ProtocolError",
                        Some(serde_json::json!({
                            "reason": format!("message exceeded {} bytes", MAX_MSG_BYTES)
                        })),
                    );
                    let _ = stream.write_all(&err_reply.to_bytes()).await;
                    let _ = stream.flush().await;
                    return Ok(());
                }

                // Process all complete NUL-delimited messages
                while let Some(nul_pos) = buffer.iter().position(|&b| b == 0x00) {
                    let message_bytes = buffer.drain(..=nul_pos).collect::<Vec<u8>>();
                    // Strip the trailing NUL by trimming the last byte.
                    let payload = &message_bytes[..message_bytes.len() - 1];

                    if payload.is_empty() {
                        continue;
                    }

                    let call: VarlinkCall = match serde_json::from_slice(payload) {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Invalid Varlink call payload: {}", e);
                            let reply =
                                VarlinkReply::err("org.varlink.service.InvalidParameter", None);
                            stream.write_all(&reply.to_bytes()).await?;
                            continue;
                        }
                    };

                    let reply = dispatch_call(&call, &handler).await;
                    stream.write_all(&reply.to_bytes()).await?;
                }
            }
        }
    }

    Ok(())
}

async fn dispatch_call(call: &VarlinkCall, handler: &Tool1Handler) -> VarlinkReply {
    if let Some(reply) = handle_service_call(&call.method, call.parameters.as_ref()) {
        return reply;
    }

    if let Some(reply) = handler.handle_call(&call.method, call.parameters.as_ref()).await {
        return reply;
    }

    VarlinkReply::err(
        "org.varlink.service.MethodNotFound",
        Some(serde_json::json!({ "method": call.method })),
    )
}

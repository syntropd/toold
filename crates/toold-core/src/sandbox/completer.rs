//! Model completers for autonomous code repair via Varlink (runtimed) or HTTP (routerd).

use crate::error::TooldError;
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

pub const DEFAULT_RUNTIMED_SOCKET: &str = "/run/syntrop/io.syntrop.Runtime1";
pub const DEFAULT_ROUTERD_HTTP: &str = "127.0.0.1:32768";

/// Trait for querying models (runtimed, routerd, or test mocks) for code repair.
pub trait ModelCompleter: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String, TooldError>;
}

/// Varlink completer connecting to runtimed (`io.syntrop.Runtime1.Generate`).
#[derive(Debug, Clone)]
pub struct VarlinkCompleter {
    pub socket_path: PathBuf,
    pub model: String,
}

impl VarlinkCompleter {
    pub fn new<P: AsRef<Path>, S: Into<String>>(socket_path: P, model: S) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            model: model.into(),
        }
    }

    pub fn with_default_socket<S: Into<String>>(model: S) -> Self {
        Self::new(DEFAULT_RUNTIMED_SOCKET, model)
    }

    async fn complete_async(&self, prompt: &str) -> Result<String, TooldError> {
        let mut stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            TooldError::Diagnostic(format!(
                "Failed to connect to runtimed at {}: {}",
                self.socket_path.display(),
                e
            ))
        })?;

        let req = json!({
            "method": "io.syntrop.Runtime1.Generate",
            "parameters": {
                "model": self.model,
                "prompt": prompt,
                "max_tokens": 1024,
                "temperature": 0.0,
                "top_k": 0,
                "top_p": 1.0,
                "seed": 0
            }
        });

        let mut bytes = serde_json::to_vec(&req)
            .map_err(|e| TooldError::Diagnostic(format!("Failed serializing request: {}", e)))?;
        bytes.push(0x00);
        stream.write_all(&bytes).await.map_err(TooldError::Io)?;

        let mut buf = Vec::new();
        let mut chunk = [0u8; 2048];
        loop {
            let n = stream.read(&mut chunk).await.map_err(TooldError::Io)?;
            if n == 0 {
                return Err(TooldError::Diagnostic(
                    "Unexpected EOF from runtimed".into(),
                ));
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.iter().position(|&b| b == 0x00) {
                let val: serde_json::Value = serde_json::from_slice(&buf[..pos]).map_err(|e| {
                    TooldError::Diagnostic(format!("Invalid JSON from runtimed: {}", e))
                })?;
                if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
                    return Err(TooldError::Diagnostic(format!("runtimed error: {}", err)));
                }
                let text = val
                    .get("parameters")
                    .and_then(|p| p.get("result"))
                    .and_then(|r| r.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(text);
            }
        }
    }
}

impl ModelCompleter for VarlinkCompleter {
    fn complete(&self, prompt: &str) -> Result<String, TooldError> {
        let handle = tokio::runtime::Handle::try_current();
        match handle {
            Ok(h) => tokio::task::block_in_place(|| h.block_on(self.complete_async(prompt))),
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(TooldError::Io)?;
                rt.block_on(self.complete_async(prompt))
            }
        }
    }
}

/// HTTP completer connecting to routerd (`/v1/chat/completions`).
#[derive(Debug, Clone)]
pub struct HttpCompleter {
    pub host_port: String,
    pub model: String,
}

impl HttpCompleter {
    pub fn new<S: Into<String>, M: Into<String>>(host_port: S, model: M) -> Self {
        Self {
            host_port: host_port.into(),
            model: model.into(),
        }
    }

    pub fn with_default_endpoint<M: Into<String>>(model: M) -> Self {
        Self::new(DEFAULT_ROUTERD_HTTP, model)
    }

    async fn complete_async(&self, prompt: &str) -> Result<String, TooldError> {
        let mut stream = TcpStream::connect(&self.host_port).await.map_err(|e| {
            TooldError::Diagnostic(format!(
                "Failed to connect to routerd at {}: {}",
                self.host_port, e
            ))
        })?;

        let body = json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 1024,
            "temperature": 0.0
        })
        .to_string();

        let req = format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            self.host_port,
            body.len(),
            body
        );

        stream
            .write_all(req.as_bytes())
            .await
            .map_err(TooldError::Io)?;
        let mut resp = String::new();
        stream
            .read_to_string(&mut resp)
            .await
            .map_err(TooldError::Io)?;

        if let Some(pos) = resp.find("\r\n\r\n") {
            let body = &resp[pos + 4..];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
                let content = val
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|choice| choice.get("message"))
                    .and_then(|msg| msg.get("content"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(content);
            }
        }

        Err(TooldError::Diagnostic(
            "Failed parsing HTTP response from routerd".into(),
        ))
    }
}

impl ModelCompleter for HttpCompleter {
    fn complete(&self, prompt: &str) -> Result<String, TooldError> {
        let handle = tokio::runtime::Handle::try_current();
        match handle {
            Ok(h) => tokio::task::block_in_place(|| h.block_on(self.complete_async(prompt))),
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(TooldError::Io)?;
                rt.block_on(self.complete_async(prompt))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completer_constructors() {
        let vc = VarlinkCompleter::with_default_socket("qwen2.5-coder-7b");
        assert_eq!(vc.model, "qwen2.5-coder-7b");
        assert_eq!(vc.socket_path, PathBuf::from(DEFAULT_RUNTIMED_SOCKET));

        let hc = HttpCompleter::with_default_endpoint("fast");
        assert_eq!(hc.model, "fast");
        assert_eq!(hc.host_port, DEFAULT_ROUTERD_HTTP);
    }
}

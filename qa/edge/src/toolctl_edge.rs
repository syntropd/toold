//! Edge QA tests for the toolctl client and subcommand handlers.
//!
//! A stub Varlink daemon answers canned replies over a temp Unix socket,
//! so every `exec_*` handler plus both `TooldClient` outcomes run fully
//! offline with no live toold.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;
    use toolctl::client::TooldClient;
    use toolctl::cmd::{exec_history, exec_info, exec_list, exec_rollback, exec_run};

    /// Canned reply envelope per method; unknown methods get MethodNotFound.
    fn canned_reply(method: &str) -> Value {
        match method {
            "org.varlink.service.GetInfo" => json!({
                "parameters": {
                    "vendor": "Stub", "product": "toold", "version": "0.0.0",
                    "url": "http://localhost/",
                    "interfaces": ["org.varlink.service"]
                }
            }),
            "org.varlink.service.GetInterfaceDescription" => json!({
                "parameters": { "description": "interface org.varlink.service" }
            }),
            "io.syntrop.Tool1.ListTools" => json!({
                "parameters": {
                    "tools": [
                        {"name": "unit.status", "description": "stub tool",
                         "mode": "ReadOnly", "timeout_ms": 2000}
                    ]
                }
            }),
            "io.syntrop.Tool1.ExecuteTool" => json!({
                "parameters": {
                    "result": {"command": "stub", "exit_code": 0,
                               "stdout": "stub-out", "stderr": "",
                               "duration_ms": 1},
                    "rollback_id": null
                }
            }),
            "io.syntrop.Tool1.Rollback" => json!({
                "parameters": {
                    "restored": {"id": "rb-stub", "target_path": "/tmp/x",
                                 "target_unit": null, "summary": "stub restore"}
                }
            }),
            "io.syntrop.Tool1.ListRollbacks" => json!({
                "parameters": {
                    "records": [{"id": "rb-stub", "target_path": "/tmp/x",
                                 "target_unit": null, "summary": "stub restore"}]
                }
            }),
            _ => json!({
                "error": "org.varlink.service.MethodNotFound",
                "parameters": { "method": method }
            }),
        }
    }

    /// Serves one request per connection until the listener is aborted.
    async fn serve_stub(listener: UnixListener) {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 1024];
                loop {
                    match stream.read(&mut chunk).await {
                        Ok(0) => return,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        Err(_) => return,
                    }
                    if buf.len() > 65536 {
                        return;
                    }
                    if let Some(pos) = buf.iter().position(|&b| b == 0x00) {
                        let req: Value = serde_json::from_slice(&buf[..pos]).unwrap_or(Value::Null);
                        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                        let mut reply = serde_json::to_vec(&canned_reply(method)).unwrap();
                        reply.push(0x00);
                        let _ = stream.write_all(&reply).await;
                        return;
                    }
                }
            });
        }
    }

    async fn start_stub() -> (tempfile::TempDir, PathBuf, tokio::task::JoinHandle<()>) {
        let tmp = tempdir().unwrap();
        let sock = tmp.path().join("stub.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let handle = tokio::spawn(serve_stub(listener));
        (tmp, sock, handle)
    }

    #[tokio::test]
    async fn test_toolctl_client_returns_parameters() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        let res = client
            .call("io.syntrop.Tool1.ListTools", None)
            .await
            .unwrap();
        assert_eq!(res["tools"][0]["name"], "unit.status");
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_client_surfaces_daemon_error() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        let err = client.call("bogus.Method", None).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("MethodNotFound"), "unexpected: {msg}");
        assert!(msg.contains("bogus.Method"), "unexpected: {msg}");
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_client_connect_failure() {
        let client = TooldClient::new("/nonexistent-dir-xyz/stub.sock");
        let err = client
            .call("io.syntrop.Tool1.ListTools", None)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Failed to connect"),
            "unexpected: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_toolctl_exec_list_modes() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        exec_list(&client, true).await.unwrap();
        exec_list(&client, false).await.unwrap();
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_exec_run_modes() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        let args = vec!["a".to_string()];
        exec_run(&client, "unit.status", &args, Some("x.service"), true)
            .await
            .unwrap();
        exec_run(&client, "unit.status", &args, None, false)
            .await
            .unwrap();
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_exec_info_modes() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        exec_info(&client, true).await.unwrap();
        exec_info(&client, false).await.unwrap();
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_exec_history_modes() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        exec_history(&client, 60, 5, true).await.unwrap();
        exec_history(&client, 60, 5, false).await.unwrap();
        stub.abort();
    }

    #[tokio::test]
    async fn test_toolctl_exec_rollback_modes() {
        let (_tmp, sock, stub) = start_stub().await;
        let client = TooldClient::new(&sock);
        exec_rollback(&client, "rb-stub", true).await.unwrap();
        exec_rollback(&client, "rb-stub", false).await.unwrap();
        stub.abort();
    }
}

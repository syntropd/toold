//! Edge QA tests for the live Varlink socket server and dispatcher.
//!
//! A real `VarlinkServer` binds a temp socket and answers raw client calls,
//! proving accept/dispatch/shutdown plus the Tool1 rollback arms end to end.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::sync::watch;
    use toold_core::journal::RollbackJournal;
    use toold_core::policy::{ToolDefinition, ToolRegistry};
    use toold_daemon::varlink::{Tool1Handler, VarlinkServer};

    /// Sends one NUL-terminated call on a fresh connection, returns the reply.
    async fn call(sock: &Path, method: &str, params: Option<Value>) -> Value {
        let mut stream = UnixStream::connect(sock).await.unwrap();
        let mut req = serde_json::to_vec(&json!({"method": method, "parameters": params})).unwrap();
        req.push(0x00);
        stream.write_all(&req).await.unwrap();
        read_reply(&mut stream).await
    }

    async fn read_reply(stream: &mut UnixStream) -> Value {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed connection before reply");
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.iter().position(|&b| b == 0x00) {
                return serde_json::from_slice(&buf[..pos]).unwrap();
            }
        }
    }

    fn spawn_server(
        sock: &Path,
        dir: &Path,
    ) -> (
        watch::Sender<bool>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    ) {
        let registry = Arc::new(ToolRegistry::empty());
        registry.register(ToolDefinition::read_only(
            "test.echo",
            "Echo service probe",
            "/usr/bin/echo",
            vec!["srv".into()],
            2000,
        ));
        let journal = Arc::new(RollbackJournal::new(dir).unwrap());
        let handler = Tool1Handler::new(registry, journal);
        let listener = UnixListener::bind(sock).unwrap();
        let (tx, rx) = watch::channel(false);
        let server = VarlinkServer::new(listener, handler, rx);
        (tx, tokio::spawn(server.run()))
    }

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempdir().unwrap();
        let sock = tmp.path().join("toold.sock");
        (tmp, sock)
    }

    #[tokio::test]
    async fn test_server_serves_info_and_tools() {
        let (_tmp, sock) = setup();
        let (_tx, server) = spawn_server(&sock, _tmp.path());
        let info = call(&sock, "org.varlink.service.GetInfo", None).await;
        assert_eq!(info["parameters"]["product"], "toold");
        let tools = call(&sock, "io.syntrop.Tool1.ListTools", None).await;
        assert_eq!(tools["parameters"]["tools"][0]["name"], "test.echo");
        server.abort();
    }

    #[tokio::test]
    async fn test_server_executes_tool_and_reports_unknown() {
        let (_tmp, sock) = setup();
        let (_tx, server) = spawn_server(&sock, _tmp.path());
        let reply = call(
            &sock,
            "io.syntrop.Tool1.ExecuteTool",
            Some(json!({"name": "test.echo", "args": ["hello"]})),
        )
        .await;
        assert_eq!(reply["parameters"]["result"]["stdout"], "srv hello");
        let missing = call(
            &sock,
            "io.syntrop.Tool1.ExecuteTool",
            Some(json!({"name": "bogus.tool", "args": []})),
        )
        .await;
        assert_eq!(missing["error"], "io.syntrop.Tool1.ToolNotFound");
        server.abort();
    }

    #[tokio::test]
    async fn test_server_rollback_arms() {
        let (_tmp, sock) = setup();
        let (_tx, server) = spawn_server(&sock, _tmp.path());
        let denied = call(
            &sock,
            "io.syntrop.Tool1.Rollback",
            Some(json!({"rollback_id": "rb-does-not-exist"})),
        )
        .await;
        assert_eq!(denied["error"], "io.syntrop.Tool1.ExecutionFailed");
        let empty = call(
            &sock,
            "io.syntrop.Tool1.ListRollbacks",
            Some(json!({"since_seconds": 60, "limit": 10})),
        )
        .await;
        assert_eq!(empty["parameters"]["records"].as_array().unwrap().len(), 0);
        server.abort();
    }

    #[tokio::test]
    async fn test_server_rejects_unknown_method_and_bad_json() {
        let (_tmp, sock) = setup();
        let (_tx, server) = spawn_server(&sock, _tmp.path());
        let missing = call(&sock, "bogus.Interface.Method", None).await;
        assert_eq!(missing["error"], "org.varlink.service.MethodNotFound");

        // Invalid JSON gets an error reply and the connection stays usable.
        let mut stream = UnixStream::connect(&sock).await.unwrap();
        stream.write_all(b"not json at all\x00").await.unwrap();
        let bad = read_reply(&mut stream).await;
        assert_eq!(bad["error"], "org.varlink.service.InvalidParameter");
        let mut req =
            serde_json::to_vec(&json!({"method": "org.varlink.service.GetInfo"})).unwrap();
        req.push(0x00);
        stream.write_all(&req).await.unwrap();
        let info = read_reply(&mut stream).await;
        assert_eq!(info["parameters"]["product"], "toold");
        server.abort();
    }

    #[tokio::test]
    async fn test_server_shuts_down_on_signal() {
        let (_tmp, sock) = setup();
        let (tx, server) = spawn_server(&sock, _tmp.path());
        let info = call(&sock, "org.varlink.service.GetInfo", None).await;
        assert_eq!(info["parameters"]["product"], "toold");
        tx.send(true).unwrap();
        let done = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert!(done.is_ok());
    }
}

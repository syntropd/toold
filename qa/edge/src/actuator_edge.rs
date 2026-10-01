//! Integration tests for io.syntrop.Actuator1 Varlink RPC and uinput emissions.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::sync::watch;
    use toold_core::actuator::{keycodes::*, UInputActuator};
    use toold_core::journal::RollbackJournal;
    use toold_core::policy::ToolRegistry;
    use toold_daemon::varlink::{Actuator1Handler, Tool1Handler, VarlinkServer};

    async fn call(sock: &Path, method: &str, params: Option<Value>) -> Value {
        let mut stream = UnixStream::connect(sock).await.unwrap();
        let mut req = serde_json::to_vec(&json!({"method": method, "parameters": params})).unwrap();
        req.push(0x00);
        stream.write_all(&req).await.unwrap();

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

    #[tokio::test]
    async fn test_actuator_rpc_and_keypress_emissions() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("actuator.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let (act, mock) = UInputActuator::mock();
        let actuator_handler = Actuator1Handler::with_actuator(act);

        let registry = Arc::new(ToolRegistry::empty());
        let journal = Arc::new(RollbackJournal::new(dir.path()).unwrap());
        let tool_handler = Tool1Handler::new(registry, journal);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server =
            VarlinkServer::new(listener, tool_handler, shutdown_rx).with_actuator(actuator_handler);

        let handle = tokio::spawn(server.run());

        // 1. SendKey
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.SendKey",
            Some(json!({ "key_code": KEY_A, "down": true })),
        )
        .await;
        assert!(reply.get("error").is_none());

        // 2. TypeText
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.TypeText",
            Some(json!({ "text": "Hi" })),
        )
        .await;
        assert!(reply.get("error").is_none());

        // 3. MoveMouse
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.MoveMouse",
            Some(json!({ "dx": 20, "dy": -15 })),
        )
        .await;
        assert!(reply.get("error").is_none());

        // 3b. MoveMouseRel alias
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.MoveMouseRel",
            Some(json!({ "dx": 5, "dy": 5 })),
        )
        .await;
        assert!(reply.get("error").is_none());

        // 3c. MoveMouseAbs
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.MoveMouseAbs",
            Some(json!({ "x": 0.5, "y": 0.75 })),
        )
        .await;
        assert!(reply.get("error").is_none());

        // 4. ClickMouse
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.ClickMouse",
            Some(json!({ "button": 1 })),
        )
        .await;
        assert!(reply.get("error").is_none());

        let events = mock.recorded_events();
        assert!(!events.is_empty());
        // Verify keypress was recorded
        assert_eq!(events[0].type_, EV_KEY);
        assert_eq!(events[0].code, KEY_A);
        assert_eq!(events[0].value, 1);

        let _ = shutdown_tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_actuator_error_when_uinput_absent() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("actuator_absent.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let actuator_handler = Actuator1Handler::absent_for_test();

        let registry = Arc::new(ToolRegistry::empty());
        let journal = Arc::new(RollbackJournal::new(dir.path()).unwrap());
        let tool_handler = Tool1Handler::new(registry, journal);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server =
            VarlinkServer::new(listener, tool_handler, shutdown_rx).with_actuator(actuator_handler);

        let handle = tokio::spawn(server.run());

        // SendKey when absent
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.SendKey",
            Some(json!({ "key_code": KEY_ENTER, "down": true })),
        )
        .await;
        assert_eq!(
            reply["error"].as_str(),
            Some("io.syntrop.Actuator1.ActuatorUnavailable")
        );

        // TypeText when absent
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.TypeText",
            Some(json!({ "text": "hello" })),
        )
        .await;
        assert_eq!(
            reply["error"].as_str(),
            Some("io.syntrop.Actuator1.ActuatorUnavailable")
        );

        // InvalidParameter
        let reply = call(
            &sock,
            "io.syntrop.Actuator1.SendKey",
            Some(json!({ "down": true })),
        )
        .await;
        assert_eq!(
            reply["error"].as_str(),
            Some("io.syntrop.Actuator1.InvalidParameter")
        );

        let reply = call(
            &sock,
            "io.syntrop.Actuator1.MoveMouseAbs",
            Some(json!({ "x": 0.5 })),
        )
        .await;
        assert_eq!(
            reply["error"].as_str(),
            Some("io.syntrop.Actuator1.InvalidParameter")
        );

        let _ = shutdown_tx.send(true);
        let _ = handle.await;
    }
}

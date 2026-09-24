//! Unit QA tests for toold Varlink framing and method dispatch.

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::tempdir;
    use toold_core::journal::RollbackJournal;
    use toold_core::policy::ToolRegistry;
    use toold_daemon::varlink::{
        handle_service_call, Tool1Handler, VarlinkReply,
    };

    #[test]
    fn test_varlink_reply_to_bytes() {
        let reply = VarlinkReply::ok(json!({ "status": "ok" }));
        let bytes = reply.to_bytes();
        assert_eq!(*bytes.last().unwrap(), 0x00);
        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(parsed["parameters"]["status"], "ok");
    }

    #[test]
    fn test_handle_service_get_info() {
        let reply = handle_service_call("org.varlink.service.GetInfo", None);
        assert!(reply.is_some());
        let params = reply.unwrap().parameters.unwrap();
        assert_eq!(params["product"], "toold");
    }

    #[test]
    fn test_handle_service_get_interface_description() {
        let params = json!({ "interface": "io.syntrop.Tool1" });
        let reply = handle_service_call(
            "org.varlink.service.GetInterfaceDescription",
            Some(&params),
        );
        assert!(reply.is_some());
        let desc = reply.unwrap().parameters.unwrap();
        assert!(desc["description"]
            .as_str()
            .unwrap()
            .contains("interface io.syntrop.Tool1"));
    }

    #[tokio::test]
    async fn test_tool1_list_tools() {
        let tmp = tempdir().unwrap();
        let registry = Arc::new(ToolRegistry::with_defaults());
        let journal = Arc::new(RollbackJournal::new(tmp.path()).unwrap());
        let handler = Tool1Handler::new(registry, journal);

        let reply = handler
            .handle_call("io.syntrop.Tool1.ListTools", None)
            .await
            .unwrap();

        assert!(reply.error.is_none());
        let tools = reply.parameters.unwrap();
        assert!(!tools["tools"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_tool1_execute_tool_success() {
        let tmp = tempdir().unwrap();
        let registry = Arc::new(ToolRegistry::empty());

        let echo_tool = toold_core::policy::ToolDefinition::read_only(
            "test.echo",
            "Echo message",
            "/usr/bin/echo",
            vec!["varlink".into()],
            2000,
        );
        registry.register(echo_tool);

        let journal = Arc::new(RollbackJournal::new(tmp.path()).unwrap());
        let handler = Tool1Handler::new(registry, journal);

        let params = json!({
            "name": "test.echo",
            "args": ["hello"]
        });

        let reply = handler
            .handle_call("io.syntrop.Tool1.ExecuteTool", Some(&params))
            .await
            .unwrap();

        assert!(reply.error.is_none());
        let res = reply.parameters.unwrap();
        assert_eq!(res["result"]["stdout"], "varlink hello");
    }
}

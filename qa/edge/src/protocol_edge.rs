//! Edge tests for Varlink framing boundaries and error states.

#[cfg(test)]
mod tests {
    use toold_daemon::varlink::{VarlinkCall, VarlinkReply};

    #[test]
    fn test_varlink_call_parsing_edge_cases() {
        let empty_call = b"{\"method\":\"org.varlink.service.GetInfo\"}";
        let parsed: Result<VarlinkCall, _> = serde_json::from_slice(empty_call);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().method, "org.varlink.service.GetInfo");

        let invalid = b"not json at all";
        let err: Result<VarlinkCall, _> = serde_json::from_slice(invalid);
        assert!(err.is_err());
    }

    #[test]
    fn test_varlink_empty_error_reply() {
        let reply = VarlinkReply::err("CustomError", None);
        let bytes = reply.to_bytes();
        assert_eq!(*bytes.last().unwrap(), 0x00);
        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(parsed["error"], "CustomError");
    }
}

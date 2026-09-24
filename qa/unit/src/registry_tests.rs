//! Unit QA tests for ToolRegistry.

#[cfg(test)]
mod tests {
    use toold_core::policy::{ToolDefinition, ToolRegistry};

    #[test]
    fn test_registry_with_defaults() {
        let registry = ToolRegistry::with_defaults();
        let tools = registry.list();
        assert!(!tools.is_empty());
        assert!(registry.get("unit.status").is_some());
        assert!(registry.get("journal.slice").is_some());
        assert!(registry.get("unit.restart").is_some());
    }

    #[test]
    fn test_custom_tool_registration() {
        let registry = ToolRegistry::empty();
        assert!(registry.list().is_empty());

        let custom = ToolDefinition::read_only(
            "custom.tool",
            "Custom diagnostic tool",
            "/usr/bin/echo",
            vec!["hello".into()],
            1000,
        );

        registry.register(custom);

        let retrieved = registry.get("custom.tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "custom.tool");
    }

    #[test]
    fn test_unknown_tool_returns_none() {
        let registry = ToolRegistry::empty();
        assert!(registry.get("nonexistent.tool").is_none());
    }
}

//! Unit QA tests for TooldConfig loading and defaults.

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;
    use toold_core::config::{
        TooldConfig, DEFAULT_SOCKET_PATH, DEFAULT_STORAGE_PATH,
    };

    #[test]
    fn test_default_config() {
        let config = TooldConfig::default();
        assert_eq!(config.storage_path.to_str().unwrap(), DEFAULT_STORAGE_PATH);
        assert_eq!(config.socket_path.to_str().unwrap(), DEFAULT_SOCKET_PATH);
        assert_eq!(config.default_timeout_ms, 10000);
        assert_eq!(config.max_output_bytes, 65536);
    }

    #[test]
    fn test_load_from_valid_toml() {
        let tmp = tempdir().unwrap();
        let conf_file = tmp.path().join("toold.conf");

        let toml_data = r#"
            storage_path = "/tmp/custom_toold"
            socket_path = "/tmp/custom_toold.sock"
            default_timeout_ms = 5000
            max_output_bytes = 32768
        "#;
        fs::write(&conf_file, toml_data).unwrap();

        let loaded = TooldConfig::load_or_default(&conf_file).unwrap();
        assert_eq!(loaded.storage_path.to_str().unwrap(), "/tmp/custom_toold");
        assert_eq!(loaded.default_timeout_ms, 5000);
        assert_eq!(loaded.max_output_bytes, 32768);
    }

    #[test]
    fn test_missing_file_fallback() {
        let tmp = tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist.conf");

        let loaded = TooldConfig::load_or_default(missing).unwrap();
        assert_eq!(loaded, TooldConfig::default());
    }
}

//! Configuration parser and global defaults.

pub mod toold_config;

pub use toold_config::{
    TooldConfig, DEFAULT_CONFIG_PATH, DEFAULT_SOCKET_PATH, DEFAULT_STORAGE_PATH,
};

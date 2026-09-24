//! Varlink protocol definitions and server implementation for toold.

pub mod protocol;
pub mod server;
pub mod service;
pub mod tool1;

pub use protocol::{VarlinkCall, VarlinkReply};
pub use server::VarlinkServer;
pub use service::{handle_service_call, IO_SYNTROP_TOOL1_INTERFACE};
pub use tool1::Tool1Handler;

//! Varlink protocol definitions and server implementation for toold.
//!
//! Provides the primary IPC transport for tool execution and actuation,
//! secured via kernel-verified SO_PEERCRED authorization.

pub mod actuator;
pub mod auth;
pub mod protocol;
pub mod server;
pub mod service;
pub mod tool1;

pub use actuator::Actuator1Handler;
pub use auth::{authorize_peer, lookup_group, TrustedGroup, UNRESOLVED_GID};
pub use protocol::{VarlinkCall, VarlinkReply};
pub use server::VarlinkServer;
pub use service::{handle_service_call, IO_SYNTROP_TOOL1_INTERFACE};
pub use tool1::Tool1Handler;

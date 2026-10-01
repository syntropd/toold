//! Diagnostic tools for native kernel inspection without external commands.

pub mod netlink;
pub mod socket_diag;

pub use socket_diag::{dump_all_tcp_sockets, run_socket_diag, TcpSocketInfo};

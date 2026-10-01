//! Native Netlink NETLINK_INET_DIAG socket enumeration and queue inspection.

use super::netlink::*;
use crate::error::TooldError;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::io::{AsRawFd, OwnedFd};

/// Telemetry and queue diagnostics for a single TCP socket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpSocketInfo {
    pub family: String,
    pub state: String,
    pub local_addr: String,
    pub local_port: u16,
    pub peer_addr: String,
    pub peer_port: u16,
    pub rx_queue: u32,
    pub tx_queue: u32,
    pub inode: u32,
    pub uid: u32,
    pub rtt_us: Option<u32>,
    pub retrans: Option<u32>,
}

/// Enumerates all TCP sockets across IPv4 and IPv6 without shelling out.
pub fn dump_all_tcp_sockets() -> Result<Vec<TcpSocketInfo>, TooldError> {
    let fd = open_netlink_diag_socket()?;
    let mut out = Vec::new();

    send_dump_request(&fd, libc::AF_INET as u8)?;
    read_netlink_responses(&fd, &mut out)?;

    send_dump_request(&fd, libc::AF_INET6 as u8)?;
    read_netlink_responses(&fd, &mut out)?;

    Ok(out)
}

fn read_netlink_responses(fd: &OwnedFd, out: &mut Vec<TcpSocketInfo>) -> Result<(), TooldError> {
    let mut buf = [0u8; 16384];
    loop {
        let n = unsafe {
            libc::recv(
                fd.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
            )
        };
        if n < 0 {
            return Err(TooldError::Diagnostic(format!(
                "Netlink recv failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        let len = n as usize;
        if len == 0 {
            break;
        }

        let mut offset = 0;
        let mut done = false;
        while offset + 16 <= len {
            let nlh = unsafe { std::ptr::read_unaligned(buf[offset..].as_ptr() as *const NlMsgHdr) };
            let msg_len = nlh.nlmsg_len as usize;
            if msg_len < 16 || offset + msg_len > len {
                break;
            }

            if nlh.nlmsg_type == NLMSG_DONE {
                done = true;
                break;
            }
            if nlh.nlmsg_type == NLMSG_ERROR {
                return Err(TooldError::Diagnostic("Netlink error response received".into()));
            }

            if msg_len >= 16 + std::mem::size_of::<InetDiagMsg>() {
                let diag = unsafe {
                    std::ptr::read_unaligned(
                        buf[offset + 16..].as_ptr() as *const InetDiagMsg
                    )
                };
                let payload_start = offset + 16 + std::mem::size_of::<InetDiagMsg>();
                let payload_end = offset + msg_len;
                let (rtt_us, retrans) = parse_rtattrs(&buf[payload_start..payload_end]);

                out.push(parse_socket_info(&diag, rtt_us, retrans));
            }

            offset += (msg_len + 3) & !3;
        }

        if done {
            break;
        }
    }
    Ok(())
}

fn parse_rtattrs(data: &[u8]) -> (Option<u32>, Option<u32>) {
    let mut rtt_us = None;
    let mut retrans = None;
    let mut pos = 0;
    while pos + 4 <= data.len() {
        let rta_len = u16::from_ne_bytes([data[pos], data[pos + 1]]) as usize;
        let rta_type = u16::from_ne_bytes([data[pos + 2], data[pos + 3]]);
        if rta_len < 4 || pos + rta_len > data.len() {
            break;
        }
        if rta_type == INET_DIAG_INFO {
            let info = &data[pos + 4..pos + rta_len];
            if info.len() >= 72 {
                rtt_us = Some(u32::from_ne_bytes([info[68], info[69], info[70], info[71]]));
            }
            if info.len() >= 40 {
                retrans = Some(u32::from_ne_bytes([info[36], info[37], info[38], info[39]]));
            }
        }
        pos += (rta_len + 3) & !3;
    }
    (rtt_us, retrans)
}

fn parse_socket_info(msg: &InetDiagMsg, rtt_us: Option<u32>, retrans: Option<u32>) -> TcpSocketInfo {
    let family = if msg.idiag_family == libc::AF_INET6 as u8 { "tcp6" } else { "tcp" };
    let state = match msg.idiag_state {
        1 => "ESTAB", 2 => "SYN-SENT", 3 => "SYN-RECV", 4 => "FIN-WAIT-1",
        5 => "FIN-WAIT-2", 6 => "TIME-WAIT", 7 => "CLOSE", 8 => "CLOSE-WAIT",
        9 => "LAST-ACK", 10 => "LISTEN", 11 => "CLOSING", _ => "UNKNOWN",
    };

    let (local_addr, peer_addr) = if msg.idiag_family == libc::AF_INET6 as u8 {
        let src_bytes: [u8; 16] = unsafe { std::mem::transmute(msg.id.idiag_src) };
        let dst_bytes: [u8; 16] = unsafe { std::mem::transmute(msg.id.idiag_dst) };
        (Ipv6Addr::from(src_bytes).to_string(), Ipv6Addr::from(dst_bytes).to_string())
    } else {
        let src_bytes: [u8; 4] = msg.id.idiag_src[0].to_ne_bytes();
        let dst_bytes: [u8; 4] = msg.id.idiag_dst[0].to_ne_bytes();
        (Ipv4Addr::from(src_bytes).to_string(), Ipv4Addr::from(dst_bytes).to_string())
    };

    TcpSocketInfo {
        family: family.into(),
        state: state.into(),
        local_addr,
        local_port: u16::from_be(msg.id.idiag_sport),
        peer_addr,
        peer_port: u16::from_be(msg.id.idiag_dport),
        rx_queue: msg.idiag_rqueue,
        tx_queue: msg.idiag_wqueue,
        inode: msg.idiag_inode,
        uid: msg.idiag_uid,
        rtt_us,
        retrans,
    }
}

/// Executes socket diagnostic and formats output table or json.
pub fn run_socket_diag(args: &[String]) -> Result<String, TooldError> {
    let sockets = dump_all_tcp_sockets()?;
    if args.iter().any(|a| a == "--json") {
        serde_json::to_string_pretty(&sockets)
            .map_err(|e| TooldError::Diagnostic(format!("JSON serialization error: {}", e)))
    } else {
        let mut table = format!(
            "{:<8} {:<10} {:<7} {:<7} {:<22} {:<22} {:<8} {:<8}\n",
            "PROTO", "STATE", "RECV-Q", "SEND-Q", "LOCAL-ADDRESS:PORT", "PEER-ADDRESS:PORT", "RTT_US", "RETRANS"
        );
        for s in &sockets {
            let local = format!("{}:{}", s.local_addr, s.local_port);
            let peer = format!("{}:{}", s.peer_addr, s.peer_port);
            let rtt = s.rtt_us.map(|r| r.to_string()).unwrap_or_else(|| "-".into());
            let ret = s.retrans.map(|r| r.to_string()).unwrap_or_else(|| "-".into());
            table.push_str(&format!(
                "{:<8} {:<10} {:<7} {:<7} {:<22} {:<22} {:<8} {:<8}\n",
                s.family, s.state, s.rx_queue, s.tx_queue, local, peer, rtt, ret
            ));
        }
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dump_all_tcp_sockets_runs_unprivileged() {
        let res = dump_all_tcp_sockets();
        assert!(res.is_ok(), "dump_all_tcp_sockets failed: {:?}", res.err());
        let sockets = res.unwrap();
        // Host typically has at least 1 listening or open TCP socket
        for s in &sockets {
            assert!(!s.state.is_empty());
        }
    }

    #[test]
    fn test_run_socket_diag_formatting() {
        let out = run_socket_diag(&[]).unwrap();
        assert!(out.contains("PROTO"));
        assert!(out.contains("STATE"));
        assert!(out.contains("LOCAL-ADDRESS:PORT"));

        let json_out = run_socket_diag(&["--json".into()]).unwrap();
        assert!(json_out.starts_with('['));
    }
}

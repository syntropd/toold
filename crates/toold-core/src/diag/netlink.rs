//! Low-level Netlink INET_DIAG socket communication and message structures.

use crate::error::TooldError;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};

pub const NETLINK_INET_DIAG: i32 = 4;
pub const SOCK_DIAG_BY_FAMILY: u16 = 20;
pub const NLM_F_REQUEST: u16 = 0x01;
pub const NLM_F_DUMP: u16 = 0x300;
pub const NLMSG_DONE: u16 = 0x03;
pub const NLMSG_ERROR: u16 = 0x02;
pub const INET_DIAG_INFO: u16 = 2;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NlMsgHdr {
    pub nlmsg_len: u32,
    pub nlmsg_type: u16,
    pub nlmsg_flags: u16,
    pub nlmsg_seq: u32,
    pub nlmsg_pid: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InetDiagSockId {
    pub idiag_sport: u16,
    pub idiag_dport: u16,
    pub idiag_src: [u32; 4],
    pub idiag_dst: [u32; 4],
    pub idiag_if: u32,
    pub idiag_cookie: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InetDiagReqV2 {
    pub sdiag_family: u8,
    pub sdiag_protocol: u8,
    pub idiag_ext: u8,
    pub pad: u8,
    pub idiag_states: u32,
    pub id: InetDiagSockId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InetDiagMsg {
    pub idiag_family: u8,
    pub idiag_state: u8,
    pub idiag_timer: u8,
    pub idiag_retrans: u8,
    pub id: InetDiagSockId,
    pub idiag_expires: u32,
    pub idiag_rqueue: u32,
    pub idiag_wqueue: u32,
    pub idiag_uid: u32,
    pub idiag_inode: u32,
}

#[repr(C)]
pub struct InetDiagDumpRequest {
    pub nlh: NlMsgHdr,
    pub req: InetDiagReqV2,
}

/// Opens an unprivileged NETLINK_INET_DIAG socket.
pub fn open_netlink_diag_socket() -> Result<OwnedFd, TooldError> {
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_RAW | libc::O_CLOEXEC,
            NETLINK_INET_DIAG,
        )
    };
    if fd < 0 {
        return Err(TooldError::Diagnostic(format!(
            "Failed opening netlink inet_diag socket: {}",
            std::io::Error::last_os_error()
        )));
    }
    // SAFETY: fd is valid and owned exclusively
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Sends an INET_DIAG dump request for the given address family (AF_INET or AF_INET6).
pub fn send_dump_request(fd: &OwnedFd, family: u8) -> Result<(), TooldError> {
    let mut dump = InetDiagDumpRequest {
        nlh: NlMsgHdr {
            nlmsg_len: std::mem::size_of::<InetDiagDumpRequest>() as u32,
            nlmsg_type: SOCK_DIAG_BY_FAMILY,
            nlmsg_flags: NLM_F_REQUEST | NLM_F_DUMP,
            nlmsg_seq: 1,
            nlmsg_pid: 0,
        },
        req: InetDiagReqV2 {
            sdiag_family: family,
            sdiag_protocol: libc::IPPROTO_TCP as u8,
            idiag_ext: (1 << (INET_DIAG_INFO - 1)),
            pad: 0,
            idiag_states: 0xFFFFFFFF,
            id: InetDiagSockId::default(),
        },
    };

    let buf = unsafe {
        std::slice::from_raw_parts(
            &mut dump as *mut _ as *const u8,
            std::mem::size_of::<InetDiagDumpRequest>(),
        )
    };

    let sent = unsafe {
        libc::send(
            fd.as_raw_fd(),
            buf.as_ptr() as *const libc::c_void,
            buf.len(),
            0,
        )
    };
    if sent < 0 {
        return Err(TooldError::Diagnostic(format!(
            "Failed to send Netlink dump request: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

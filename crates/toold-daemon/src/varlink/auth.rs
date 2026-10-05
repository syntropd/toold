//! Peer-credential authorization for toold Varlink connections.
//!
//! rustix reads `SO_PEERCRED` so even if the socket file mode or systemd
//! activation is misconfigured the daemon only accepts clients whose uid
//! is root or whose gid matches the configured trusted group.

use anyhow::{anyhow, Result};
use std::os::fd::AsFd;

/// Sentinel for "trusted group could not be resolved at startup".
pub const UNRESOLVED_GID: u32 = u32::MAX;

/// Group identifier used for the trusted-client ACL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedGroup {
    gid: u32,
}

impl TrustedGroup {
    /// Builds a `TrustedGroup` from a raw gid.
    pub fn from_gid(gid: u32) -> Self {
        Self { gid }
    }

    /// Returns the underlying gid.
    pub fn gid(self) -> u32 {
        self.gid
    }

    /// Returns true when the lookup failed or the group could not be resolved.
    pub fn is_unresolved(self) -> bool {
        self.gid == UNRESOLVED_GID
    }
}

/// Resolves an NSS group name to its gid, or `UNRESOLVED_GID` on failure.
pub fn lookup_group(name: &str) -> u32 {
    let c_name = match std::ffi::CString::new(name) {
        Ok(s) => s,
        Err(_) => return UNRESOLVED_GID,
    };
    let mut buf_size: usize = unsafe {
        let s = libc::sysconf(libc::_SC_GETGR_R_SIZE_MAX);
        if s > 0 {
            s as usize
        } else {
            1024
        }
    }
    .max(1024);

    let mut grp = std::mem::MaybeUninit::<libc::group>::uninit();
    let mut result: *mut libc::group = std::ptr::null_mut();
    loop {
        let mut buf = vec![0u8; buf_size];
        let err = unsafe {
            libc::getgrnam_r(
                c_name.as_ptr(),
                grp.as_mut_ptr(),
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                &mut result,
            )
        };
        if err == libc::ERANGE {
            buf_size = buf_size.saturating_mul(2);
            if buf_size > 64 * 1024 {
                return UNRESOLVED_GID;
            }
            continue;
        }
        if err != 0 || result.is_null() {
            return UNRESOLVED_GID;
        }
        return unsafe { (*result).gr_gid as u32 };
    }
}

/// Checks whether a user (by uid) belongs to the target gid.
pub fn is_user_in_group(uid: u32, target_gid: u32) -> bool {
    let mut pwd = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    let mut buf = vec![0u8; 2048];
    let err = unsafe {
        libc::getpwuid_r(
            uid as libc::uid_t,
            pwd.as_mut_ptr(),
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            &mut result,
        )
    };
    if err != 0 || result.is_null() {
        return false;
    }
    let username = unsafe { (*result).pw_name };
    let primary_gid = unsafe { (*result).pw_gid };
    if primary_gid as u32 == target_gid {
        return true;
    }

    let mut ngroups: libc::c_int = 64;
    let mut groups = vec![0 as libc::gid_t; 64];
    let res = unsafe {
        libc::getgrouplist(
            username,
            primary_gid,
            groups.as_mut_ptr(),
            &mut ngroups,
        )
    };
    if res == -1 && ngroups > 64 {
        groups.resize(ngroups as usize, 0);
        let res2 = unsafe {
            libc::getgrouplist(
                username,
                primary_gid,
                groups.as_mut_ptr(),
                &mut ngroups,
            )
        };
        if res2 == -1 {
            return false;
        }
    }
    groups[..ngroups as usize].contains(&target_gid)
}

/// Returns `Ok(())` when the peer is root or in the trusted group, else Err.
pub fn authorize_peer<Fd: AsFd>(stream: Fd, trusted: TrustedGroup) -> Result<()> {
    let cred = rustix::net::sockopt::get_socket_peercred(stream)
        .map_err(|e| anyhow!("SO_PEERCRED failed: {}", e))?;
    let uid = cred.uid.as_raw();
    let gid = cred.gid.as_raw();
    if uid == 0 {
        return Ok(());
    }
    if trusted.is_unresolved() {
        return Err(anyhow!(
            "peer uid={} gid={} rejected: trusted group not resolved at startup",
            uid,
            gid
        ));
    }
    if gid == trusted.gid() || is_user_in_group(uid, trusted.gid()) {
        Ok(())
    } else {
        Err(anyhow!(
            "peer uid={} gid={} not in trusted group gid={}",
            uid,
            gid,
            trusted.gid()
        ))
    }
}

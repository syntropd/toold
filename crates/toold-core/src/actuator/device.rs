//! Hardware initialization and uinput device capability registration.

use super::event::{
    InputAbsInfo, InputId, UInputAbsSetup, UInputSetup,
    BUS_USB, UI_ABS_SETUP, UI_DEV_CREATE, UI_DEV_DESTROY, UI_DEV_SETUP,
    UI_SET_ABSBIT, UI_SET_EVBIT, UI_SET_KEYBIT, UI_SET_RELBIT,
};
use super::keycodes::{
    ABS_X, ABS_Y, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, EV_ABS, EV_KEY,
    EV_REL, REL_WHEEL, REL_X, REL_Y,
};
use crate::error::TooldError;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

/// Opens the specified uinput device node with non-blocking I/O and configures HID capabilities.
pub fn open_uinput_device(path: &Path) -> Result<File, TooldError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| {
            TooldError::Actuator(format!("Failed to open uinput at {}: {}", path.display(), e))
        })?;

    let fd = file.as_raw_fd();
    verify_nonblocking(fd)?;
    configure_capabilities(fd)?;
    create_device(fd)?;

    Ok(file)
}

fn verify_nonblocking(fd: RawFd) -> Result<(), TooldError> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(TooldError::Actuator("Failed to query file status flags".into()));
    }
    if flags & libc::O_NONBLOCK == 0 {
        return Err(TooldError::Actuator("uinput file descriptor is not non-blocking".into()));
    }
    Ok(())
}

fn configure_capabilities(fd: RawFd) -> Result<(), TooldError> {
    unsafe {
        // 1. Enable EV_SYN, EV_KEY and standard keyboard keys
        checked_ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_ulong)?;
        for key in 1u16..=248 {
            checked_ioctl(fd, UI_SET_KEYBIT, key as libc::c_ulong)?;
        }

        // 2. Enable EV_REL for relative mouse movements and wheel
        checked_ioctl(fd, UI_SET_EVBIT, EV_REL as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_RELBIT, REL_X as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_RELBIT, REL_Y as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_RELBIT, REL_WHEEL as libc::c_ulong)?;

        // Mouse buttons
        checked_ioctl(fd, UI_SET_KEYBIT, BTN_LEFT as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_KEYBIT, BTN_RIGHT as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_KEYBIT, BTN_MIDDLE as libc::c_ulong)?;

        // 3. Enable EV_ABS for normalized absolute pointer coordinates
        checked_ioctl(fd, UI_SET_EVBIT, EV_ABS as libc::c_ulong)?;
        checked_ioctl(fd, UI_SET_ABSBIT, ABS_X as libc::c_ulong)?;
        let abs_x = UInputAbsSetup {
            code: ABS_X,
            absinfo: InputAbsInfo {
                value: 0,
                minimum: 0,
                maximum: 32767,
                fuzz: 0,
                flat: 0,
                resolution: 1,
            },
        };
        checked_ioctl_ptr(fd, UI_ABS_SETUP, &abs_x as *const _ as *const libc::c_void)?;

        checked_ioctl(fd, UI_SET_ABSBIT, ABS_Y as libc::c_ulong)?;
        let abs_y = UInputAbsSetup {
            code: ABS_Y,
            absinfo: InputAbsInfo {
                value: 0,
                minimum: 0,
                maximum: 32767,
                fuzz: 0,
                flat: 0,
                resolution: 1,
            },
        };
        checked_ioctl_ptr(fd, UI_ABS_SETUP, &abs_y as *const _ as *const libc::c_void)?;
    }
    Ok(())
}

fn create_device(fd: RawFd) -> Result<(), TooldError> {
    let mut name = [0u8; 80];
    let dev_name = b"syntrop-virtual-actuator";
    name[..dev_name.len()].copy_from_slice(dev_name);

    let setup = UInputSetup {
        id: InputId {
            bustype: BUS_USB,
            vendor: 0x1d6b, // Linux Foundation
            product: 0x0001,
            version: 1,
        },
        name,
        ff_effects_max: 0,
    };

    unsafe {
        checked_ioctl_ptr(fd, UI_DEV_SETUP, &setup as *const _ as *const libc::c_void)?;
        checked_ioctl(fd, UI_DEV_CREATE, 0)?;
    }
    Ok(())
}

pub fn destroy_device(fd: RawFd) -> Result<(), TooldError> {
    unsafe {
        checked_ioctl(fd, UI_DEV_DESTROY, 0)?;
    }
    Ok(())
}

unsafe fn checked_ioctl(fd: RawFd, req: u64, arg: libc::c_ulong) -> Result<(), TooldError> {
    let res = libc::ioctl(fd, req, arg);
    if res < 0 {
        let err = std::io::Error::last_os_error();
        return Err(TooldError::Actuator(format!("ioctl 0x{:x} failed: {}", req, err)));
    }
    Ok(())
}

unsafe fn checked_ioctl_ptr(fd: RawFd, req: u64, ptr: *const libc::c_void) -> Result<(), TooldError> {
    let res = libc::ioctl(fd, req, ptr);
    if res < 0 {
        let err = std::io::Error::last_os_error();
        return Err(TooldError::Actuator(format!("ioctl 0x{:x} failed: {}", req, err)));
    }
    Ok(())
}

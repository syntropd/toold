//! Linux uinput ioctl definitions and input event structs.

pub const UI_DEV_CREATE: u64 = 0x5501;
pub const UI_DEV_DESTROY: u64 = 0x5502;
pub const UI_DEV_SETUP: u64 = 0x405c5503;
pub const UI_ABS_SETUP: u64 = 0x401c5504;
pub const UI_SET_EVBIT: u64 = 0x40045564;
pub const UI_SET_KEYBIT: u64 = 0x40045565;
pub const UI_SET_RELBIT: u64 = 0x40045566;
pub const UI_SET_ABSBIT: u64 = 0x40045567;

pub const BUS_USB: u16 = 0x03;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputId {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UInputSetup {
    pub id: InputId,
    pub name: [u8; 80],
    pub ff_effects_max: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InputAbsInfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UInputAbsSetup {
    pub code: u16,
    pub absinfo: InputAbsInfo,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawInputEvent {
    pub time: libc::timeval,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl PartialEq for RawInputEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time.tv_sec == other.time.tv_sec
            && self.time.tv_usec == other.time.tv_usec
            && self.type_ == other.type_
            && self.code == other.code
            && self.value == other.value
    }
}

impl Eq for RawInputEvent {}

impl RawInputEvent {
    pub fn new(type_: u16, code: u16, value: i32) -> Self {
        Self {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_,
            code,
            value,
        }
    }

    pub fn to_bytes(&self) -> [u8; std::mem::size_of::<RawInputEvent>()] {
        // SAFETY: RawInputEvent contains plain old data types (POD) with standard C layout.
        unsafe { std::mem::transmute_copy(self) }
    }
}

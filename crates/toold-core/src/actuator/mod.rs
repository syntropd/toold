//! Linux /dev/uinput virtual HID actuator engine.
//!
//! Provides virtual keyboard input, text typing, relative/absolute mouse
//! pointer displacement, and mouse button click emulation.

pub mod device;
pub mod event;
pub mod keyboard;
pub mod keycodes;
pub mod mouse;
pub mod sink;

use crate::error::TooldError;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::sync::Arc;

pub use event::RawInputEvent;
pub use keycodes::*;
pub use sink::{DeviceSink, FileSink, MockSink};

pub const DEFAULT_UINPUT_PATH: &str = "/dev/uinput";

/// Virtual HID actuator controlling kernel uinput device nodes.
pub struct UInputActuator {
    sink: Arc<dyn DeviceSink>,
    raw_fd: Option<RawFd>,
}

impl UInputActuator {
    /// Opens the default `/dev/uinput` node with non-blocking I/O.
    pub fn open() -> Result<Self, TooldError> {
        Self::open_path(Path::new(DEFAULT_UINPUT_PATH))
    }

    /// Opens a custom uinput device node path.
    pub fn open_path(path: &Path) -> Result<Self, TooldError> {
        let file = device::open_uinput_device(path)?;
        let raw_fd = file.as_raw_fd();
        let sink = Arc::new(FileSink::new(file));
        Ok(Self {
            sink,
            raw_fd: Some(raw_fd),
        })
    }

    /// Creates an actuator using an arbitrary sink (such as `MockSink`).
    pub fn from_sink(sink: Arc<dyn DeviceSink>) -> Self {
        Self { sink, raw_fd: None }
    }

    /// Creates an actuator backed by an in-memory `MockSink` for testing.
    pub fn mock() -> (Self, MockSink) {
        let mock = MockSink::new();
        let act = Self::from_sink(Arc::new(mock.clone()));
        (act, mock)
    }

    /// Sends a raw key event (key down or key up).
    pub fn send_key(&self, key_code: u16, down: bool) -> Result<(), TooldError> {
        keyboard::send_key(self.sink.as_ref(), key_code, down)
    }

    /// Types an ASCII/UTF-8 string with automatic shift modifiers.
    pub fn type_text(&self, text: &str) -> Result<(), TooldError> {
        keyboard::type_text(self.sink.as_ref(), text)
    }

    /// Moves the mouse pointer by relative delta offsets in pixels.
    pub fn move_mouse_rel(&self, dx: i32, dy: i32) -> Result<(), TooldError> {
        mouse::move_mouse_rel(self.sink.as_ref(), dx, dy)
    }

    /// Moves the mouse pointer to normalized absolute coordinates [0.0, 1.0].
    pub fn move_mouse_abs(&self, x: f32, y: f32) -> Result<(), TooldError> {
        mouse::move_mouse_abs(self.sink.as_ref(), x, y)
    }

    /// Performs a mouse button click (down then up).
    pub fn click_mouse(&self, button: u16) -> Result<(), TooldError> {
        mouse::click_mouse(self.sink.as_ref(), button)
    }
}

impl Drop for UInputActuator {
    fn drop(&mut self) {
        if let Some(fd) = self.raw_fd {
            let _ = device::destroy_device(fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_send_key() {
        let (act, mock) = UInputActuator::mock();
        act.send_key(KEY_A, true).unwrap();
        let events = mock.recorded_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].type_, EV_KEY);
        assert_eq!(events[0].code, KEY_A);
        assert_eq!(events[0].value, 1);
        assert_eq!(events[1].type_, EV_SYN);
        assert_eq!(events[1].code, SYN_REPORT);
    }

    #[test]
    fn test_mock_type_text_uppercase() {
        let (act, mock) = UInputActuator::mock();
        act.type_text("A").unwrap();
        let events = mock.recorded_events();
        // Shift down, syn, KeyA down, syn, KeyA up, syn, Shift up, syn
        assert_eq!(events.len(), 8);
        assert_eq!(events[0].code, KEY_LEFTSHIFT);
        assert_eq!(events[0].value, 1);
        assert_eq!(events[2].code, KEY_A);
        assert_eq!(events[2].value, 1);
        assert_eq!(events[4].code, KEY_A);
        assert_eq!(events[4].value, 0);
        assert_eq!(events[6].code, KEY_LEFTSHIFT);
        assert_eq!(events[6].value, 0);
    }

    #[test]
    fn test_mock_mouse_move_and_click() {
        let (act, mock) = UInputActuator::mock();
        act.move_mouse_rel(10, -5).unwrap();
        act.move_mouse_abs(0.5, 0.5).unwrap();
        act.click_mouse(BTN_LEFT).unwrap();

        let events = mock.recorded_events();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_missing_device_path_fails_gracefully() {
        let res = UInputActuator::open_path(Path::new("/nonexistent/syntrop/uinput"));
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), TooldError::Actuator(_)));
    }
}

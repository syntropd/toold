//! Virtual keyboard primitive operations.

use super::event::RawInputEvent;
use super::keycodes::{char_to_keycode, EV_KEY, EV_SYN, KEY_LEFTSHIFT, SYN_REPORT};
use super::sink::DeviceSink;
use crate::error::TooldError;

/// Emits a single key event (press or release) followed by a synchronization report.
pub fn send_key(sink: &dyn DeviceSink, key_code: u16, down: bool) -> Result<(), TooldError> {
    let value = if down { 1 } else { 0 };
    sink.emit(RawInputEvent::new(EV_KEY, key_code, value))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
    sink.sync()?;
    Ok(())
}

/// Types UTF-8/ASCII text by mapping characters to evdev keycodes and shift states.
pub fn type_text(sink: &dyn DeviceSink, text: &str) -> Result<(), TooldError> {
    for ch in text.chars() {
        let (keycode, need_shift) = char_to_keycode(ch).ok_or_else(|| {
            TooldError::Actuator(format!(
                "Unsupported character '{}' for keyboard emission",
                ch
            ))
        })?;

        if need_shift {
            sink.emit(RawInputEvent::new(EV_KEY, KEY_LEFTSHIFT, 1))?;
            sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        }

        sink.emit(RawInputEvent::new(EV_KEY, keycode, 1))?;
        sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;

        sink.emit(RawInputEvent::new(EV_KEY, keycode, 0))?;
        sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;

        if need_shift {
            sink.emit(RawInputEvent::new(EV_KEY, KEY_LEFTSHIFT, 0))?;
            sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        }
    }
    sink.sync()?;
    Ok(())
}

//! Virtual mouse pointer and button primitive operations.

use super::event::RawInputEvent;
use super::keycodes::{ABS_X, ABS_Y, EV_ABS, EV_KEY, EV_REL, EV_SYN, REL_X, REL_Y, SYN_REPORT};
use super::sink::DeviceSink;
use crate::error::TooldError;

/// Emits relative mouse pointer displacement in pixels.
pub fn move_mouse_rel(sink: &dyn DeviceSink, dx: i32, dy: i32) -> Result<(), TooldError> {
    if dx != 0 {
        sink.emit(RawInputEvent::new(EV_REL, REL_X, dx))?;
    }
    if dy != 0 {
        sink.emit(RawInputEvent::new(EV_REL, REL_Y, dy))?;
    }
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
    sink.sync()?;
    Ok(())
}

/// Emits absolute mouse pointer coordinates normalized in [0.0, 1.0].
pub fn move_mouse_abs(sink: &dyn DeviceSink, x: f32, y: f32) -> Result<(), TooldError> {
    let clamped_x = x.clamp(0.0, 1.0);
    let clamped_y = y.clamp(0.0, 1.0);

    let abs_x = (clamped_x * 32767.0).round() as i32;
    let abs_y = (clamped_y * 32767.0).round() as i32;

    sink.emit(RawInputEvent::new(EV_ABS, ABS_X, abs_x))?;
    sink.emit(RawInputEvent::new(EV_ABS, ABS_Y, abs_y))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
    sink.sync()?;
    Ok(())
}

/// Emits a full mouse click (button press followed by release).
pub fn click_mouse(sink: &dyn DeviceSink, button: u16) -> Result<(), TooldError> {
    sink.emit(RawInputEvent::new(EV_KEY, button, 1))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;

    sink.emit(RawInputEvent::new(EV_KEY, button, 0))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
    sink.sync()?;
    Ok(())
}

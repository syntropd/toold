//! Virtual mouse pointer and button primitive operations.

use super::event::RawInputEvent;
use super::keycodes::{
    ABS_X, ABS_Y, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, EV_ABS, EV_KEY, EV_REL, EV_SYN, REL_X, REL_Y,
    SYN_REPORT,
};
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
    if !x.is_finite() || !y.is_finite() {
        return Err(TooldError::Actuator(
            "Normalized mouse coordinates must be finite numbers".into(),
        ));
    }
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
/// Normalizes 1 -> BTN_LEFT, 2 -> BTN_RIGHT, 3 -> BTN_MIDDLE.
pub fn click_mouse(sink: &dyn DeviceSink, button: u16) -> Result<(), TooldError> {
    let evdev_btn = match button {
        1 => BTN_LEFT,
        2 => BTN_RIGHT,
        3 => BTN_MIDDLE,
        _ => button,
    };

    sink.emit(RawInputEvent::new(EV_KEY, evdev_btn, 1))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;

    sink.emit(RawInputEvent::new(EV_KEY, evdev_btn, 0))?;
    sink.emit(RawInputEvent::new(EV_SYN, SYN_REPORT, 0))?;
    sink.sync()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actuator::sink::MockSink;

    #[test]
    fn test_move_mouse_abs_nan_fails() {
        let sink = MockSink::new();
        assert!(move_mouse_abs(&sink, f32::NAN, 0.5).is_err());
        assert!(move_mouse_abs(&sink, 0.5, f32::INFINITY).is_err());
    }

    #[test]
    fn test_move_mouse_abs_clamping() {
        let sink = MockSink::new();
        move_mouse_abs(&sink, -0.5, 1.5).unwrap();
        let events = sink.recorded_events();
        assert_eq!(events[0].value, 0);
        assert_eq!(events[1].value, 32767);
    }

    #[test]
    fn test_click_mouse_normalizes_button_indices() {
        let sink = MockSink::new();
        click_mouse(&sink, 1).unwrap();
        let events = sink.recorded_events();
        assert_eq!(events[0].code, BTN_LEFT);
        assert_eq!(events[2].code, BTN_LEFT);

        sink.clear();
        click_mouse(&sink, 2).unwrap();
        let events = sink.recorded_events();
        assert_eq!(events[0].code, BTN_RIGHT);

        sink.clear();
        click_mouse(&sink, 3).unwrap();
        let events = sink.recorded_events();
        assert_eq!(events[0].code, BTN_MIDDLE);
    }
}

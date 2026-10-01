//! Handler for io.syntrop.Actuator1 Varlink interface.

use crate::varlink::protocol::VarlinkReply;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use toold_core::actuator::UInputActuator;

/// Handles Varlink RPC method dispatches for io.syntrop.Actuator1.
#[derive(Clone)]
pub struct Actuator1Handler {
    actuator: Arc<Mutex<Option<UInputActuator>>>,
    force_absent: bool,
}

impl Default for Actuator1Handler {
    fn default() -> Self {
        Self::new()
    }
}

impl Actuator1Handler {
    /// Creates a handler that lazily connects to `/dev/uinput`.
    pub fn new() -> Self {
        Self {
            actuator: Arc::new(Mutex::new(None)),
            force_absent: false,
        }
    }

    /// Creates a handler with an existing, pre-configured actuator (e.g. mock).
    pub fn with_actuator(actuator: UInputActuator) -> Self {
        Self {
            actuator: Arc::new(Mutex::new(Some(actuator))),
            force_absent: false,
        }
    }

    /// Creates a handler configured to simulate `/dev/uinput` absence for testing.
    pub fn absent_for_test() -> Self {
        Self {
            actuator: Arc::new(Mutex::new(None)),
            force_absent: true,
        }
    }

    fn get_or_open_actuator(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<UInputActuator>>, VarlinkReply> {
        let mut guard = self.actuator.lock().map_err(|e| {
            VarlinkReply::err(
                "io.syntrop.Actuator1.DeviceError",
                Some(json!({ "reason": format!("Lock poisoned: {}", e) })),
            )
        })?;

        if self.force_absent {
            return Err(VarlinkReply::err(
                "io.syntrop.Actuator1.ActuatorUnavailable",
                Some(json!({ "reason": "/dev/uinput is absent or inaccessible" })),
            ));
        }

        if guard.is_none() {
            match UInputActuator::open() {
                Ok(act) => *guard = Some(act),
                Err(e) => {
                    return Err(VarlinkReply::err(
                        "io.syntrop.Actuator1.ActuatorUnavailable",
                        Some(json!({ "reason": e.to_string() })),
                    ));
                }
            }
        }

        Ok(guard)
    }

    /// Dispatches incoming io.syntrop.Actuator1 method calls.
    pub async fn handle_call(&self, method: &str, params: Option<&Value>) -> Option<VarlinkReply> {
        match method {
            "io.syntrop.Actuator1.SendKey" => Some(self.handle_send_key(params)),
            "io.syntrop.Actuator1.TypeText" => Some(self.handle_type_text(params)),
            "io.syntrop.Actuator1.MoveMouse" => Some(self.handle_move_mouse(params)),
            "io.syntrop.Actuator1.ClickMouse" => Some(self.handle_click_mouse(params)),
            _ => None,
        }
    }

    fn handle_send_key(&self, params: Option<&Value>) -> VarlinkReply {
        let p = match params {
            Some(p) => p,
            None => return invalid_param("parameters"),
        };
        let key_code = match p.get("key_code").and_then(|v| v.as_i64()) {
            Some(c) if c >= 0 && c <= u16::MAX as i64 => c as u16,
            _ => return invalid_param("key_code"),
        };
        let down = match p.get("down").and_then(|v| v.as_bool()) {
            Some(d) => d,
            _ => return invalid_param("down"),
        };

        let guard = match self.get_or_open_actuator() {
            Ok(g) => g,
            Err(reply) => return reply,
        };
        let act = match guard.as_ref() {
            Some(a) => a,
            None => return actuator_unavailable("Actuator not initialized"),
        };

        match act.send_key(key_code, down) {
            Ok(()) => VarlinkReply::ok(json!({})),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Actuator1.DeviceError",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }

    fn handle_type_text(&self, params: Option<&Value>) -> VarlinkReply {
        let p = match params {
            Some(p) => p,
            None => return invalid_param("parameters"),
        };
        let text = match p.get("text").and_then(|v| v.as_str()) {
            Some(t) => t,
            _ => return invalid_param("text"),
        };

        let guard = match self.get_or_open_actuator() {
            Ok(g) => g,
            Err(reply) => return reply,
        };
        let act = match guard.as_ref() {
            Some(a) => a,
            None => return actuator_unavailable("Actuator not initialized"),
        };

        match act.type_text(text) {
            Ok(()) => VarlinkReply::ok(json!({})),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Actuator1.DeviceError",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }

    fn handle_move_mouse(&self, params: Option<&Value>) -> VarlinkReply {
        let p = match params {
            Some(p) => p,
            None => return invalid_param("parameters"),
        };
        let dx = match p.get("dx").and_then(|v| v.as_i64()) {
            Some(x) => x as i32,
            _ => return invalid_param("dx"),
        };
        let dy = match p.get("dy").and_then(|v| v.as_i64()) {
            Some(y) => y as i32,
            _ => return invalid_param("dy"),
        };

        let guard = match self.get_or_open_actuator() {
            Ok(g) => g,
            Err(reply) => return reply,
        };
        let act = match guard.as_ref() {
            Some(a) => a,
            None => return actuator_unavailable("Actuator not initialized"),
        };

        match act.move_mouse_rel(dx, dy) {
            Ok(()) => VarlinkReply::ok(json!({})),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Actuator1.DeviceError",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }

    fn handle_click_mouse(&self, params: Option<&Value>) -> VarlinkReply {
        let p = match params {
            Some(p) => p,
            None => return invalid_param("parameters"),
        };
        let button = match p.get("button").and_then(|v| v.as_i64()) {
            Some(b) if b >= 0 && b <= u16::MAX as i64 => b as u16,
            _ => return invalid_param("button"),
        };

        let guard = match self.get_or_open_actuator() {
            Ok(g) => g,
            Err(reply) => return reply,
        };
        let act = match guard.as_ref() {
            Some(a) => a,
            None => return actuator_unavailable("Actuator not initialized"),
        };

        match act.click_mouse(button) {
            Ok(()) => VarlinkReply::ok(json!({})),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Actuator1.DeviceError",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }
}

fn invalid_param(param: &str) -> VarlinkReply {
    VarlinkReply::err(
        "io.syntrop.Actuator1.InvalidParameter",
        Some(json!({ "parameter": param })),
    )
}

fn actuator_unavailable(reason: &str) -> VarlinkReply {
    VarlinkReply::err(
        "io.syntrop.Actuator1.ActuatorUnavailable",
        Some(json!({ "reason": reason })),
    )
}

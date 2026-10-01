//! Device event sinks for real uinput files and mock test assertions.

use super::event::RawInputEvent;
use crate::error::TooldError;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Sink interface for emitting hardware evdev events.
pub trait DeviceSink: Send + Sync {
    /// Writes a single evdev event to the underlying device.
    fn emit(&self, event: RawInputEvent) -> Result<(), TooldError>;

    /// Flushes any pending device buffers.
    fn sync(&self) -> Result<(), TooldError>;
}

/// Real file sink writing events to a `/dev/uinput` file descriptor.
pub struct FileSink {
    file: Mutex<File>,
}

impl FileSink {
    pub fn new(file: File) -> Self {
        Self {
            file: Mutex::new(file),
        }
    }
}

impl DeviceSink for FileSink {
    fn emit(&self, event: RawInputEvent) -> Result<(), TooldError> {
        let mut guard = self.file.lock().map_err(|e| {
            TooldError::Actuator(format!("FileSink lock poisoned: {}", e))
        })?;
        let bytes = event.to_bytes();
        guard.write_all(&bytes).map_err(TooldError::Io)
    }

    fn sync(&self) -> Result<(), TooldError> {
        let mut guard = self.file.lock().map_err(|e| {
            TooldError::Actuator(format!("FileSink lock poisoned: {}", e))
        })?;
        guard.flush().map_err(TooldError::Io)
    }
}

/// In-memory mock sink recording emitted events for integration tests.
#[derive(Debug, Default, Clone)]
pub struct MockSink {
    events: Arc<Mutex<Vec<RawInputEvent>>>,
}

impl MockSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn recorded_events(&self) -> Vec<RawInputEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.events.lock() {
            g.clear();
        }
    }
}

impl DeviceSink for MockSink {
    fn emit(&self, event: RawInputEvent) -> Result<(), TooldError> {
        let mut guard = self.events.lock().map_err(|e| {
            TooldError::Actuator(format!("MockSink lock poisoned: {}", e))
        })?;
        guard.push(event);
        Ok(())
    }

    fn sync(&self) -> Result<(), TooldError> {
        Ok(())
    }
}

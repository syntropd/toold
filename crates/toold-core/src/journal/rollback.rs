//! Rollback journal and state restoration engine.
//!
//! Stores pre-execution state snapshots and applies restorations upon failure.

use crate::error::TooldError;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Historical rollback snapshot record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRecord {
    /// Unique rollback snapshot identifier.
    pub id: String,
    /// Snapshot creation timestamp in microseconds.
    pub timestamp_us: u64,
    /// Target file path that was modified, if applicable.
    pub target_path: Option<PathBuf>,
    /// Previous raw content of the target file before modification.
    pub previous_content: Option<String>,
    /// Target systemd unit associated with the action, if applicable.
    pub target_unit: Option<String>,
    /// Summary of the action that required this snapshot.
    pub summary: String,
}

/// Rollback journal manager on persistent storage.
pub struct RollbackJournal {
    log_path: PathBuf,
    lock: Mutex<()>,
}

impl RollbackJournal {
    /// Initializes a rollback journal under the designated directory.
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self, TooldError> {
        let dir = storage_dir.as_ref();
        fs::create_dir_all(dir)?;
        let log_path = dir.join("rollback.jsonl");

        if !log_path.exists() {
            File::create(&log_path)?;
        }

        Ok(Self {
            log_path,
            lock: Mutex::new(()),
        })
    }

    /// Captures the pre-modification state of a target file.
    pub fn snapshot_file<P: AsRef<Path>>(
        &self,
        target_path: P,
        target_unit: Option<&str>,
        summary: &str,
    ) -> Result<RollbackRecord, TooldError> {
        let p = target_path.as_ref();
        let prev_content = if p.exists() {
            Some(fs::read_to_string(p)?)
        } else {
            None
        };

        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(1);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);

        let record = RollbackRecord {
            id: format!("rb-{}-{}", now_us, seq),
            timestamp_us: now_us,
            target_path: Some(p.to_path_buf()),
            previous_content: prev_content,
            target_unit: target_unit.map(ToString::to_string),
            summary: summary.to_string(),
        };

        self.append(&record)?;
        Ok(record)
    }

    fn append(&self, record: &RollbackRecord) -> Result<(), TooldError> {
        let _guard = self.lock.lock().map_err(|_| {
            TooldError::Journal("Rollback mutex poisoned".into())
        })?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let mut line = serde_json::to_vec(record)
            .map_err(|e| TooldError::Journal(e.to_string()))?;
        line.push(b'\n');

        file.write_all(&line)?;
        file.flush()?;
        Ok(())
    }

    /// Reverts a prior action by restoring file content recorded in a snapshot.
    pub fn apply_rollback(&self, record_id: &str) -> Result<RollbackRecord, TooldError> {
        let records = self.list_records(0, usize::MAX)?;
        let record = records
            .into_iter()
            .find(|r| r.id == record_id)
            .ok_or_else(|| TooldError::Journal(format!("Snapshot {} not found", record_id)))?;

        if let Some(target_path) = &record.target_path {
            match &record.previous_content {
                Some(content) => {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(target_path, content)?;
                }
                None => {
                    if target_path.exists() {
                        fs::remove_file(target_path)?;
                    }
                }
            }
        }

        Ok(record)
    }

    /// Lists historical rollback snapshot records.
    pub fn list_records(&self, since_us: u64, limit: usize) -> Result<Vec<RollbackRecord>, TooldError> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path)?;
        let mut reader = BufReader::with_capacity(32768, file);
        let mut records = Vec::new();
        let mut raw_line = Vec::new();

        while let Ok(n) = reader.read_until(b'\n', &mut raw_line) {
            if n == 0 {
                break;
            }

            let line = String::from_utf8_lossy(&raw_line);
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Ok(rec) = serde_json::from_str::<RollbackRecord>(trimmed) {
                    if rec.timestamp_us >= since_us {
                        records.push(rec);
                        if records.len() >= limit {
                            break;
                        }
                    }
                }
            }
            raw_line.clear();
        }

        Ok(records)
    }
}

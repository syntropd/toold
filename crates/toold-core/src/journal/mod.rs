//! Rollback snapshot journaling and state recovery.

pub mod rollback;

pub use rollback::{RollbackJournal, RollbackRecord};

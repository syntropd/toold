//! Unit QA tests for RollbackJournal and state recovery.

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;
    use toold_core::journal::RollbackJournal;

    #[test]
    fn test_snapshot_and_rollback_existing_file() {
        let tmp = tempdir().unwrap();
        let target_file = tmp.path().join("target.conf");
        fs::write(&target_file, "original_content=1\n").unwrap();

        let journal = RollbackJournal::new(tmp.path()).unwrap();

        // Take snapshot
        let record = journal
            .snapshot_file(&target_file, Some("test.service"), "Update setting")
            .unwrap();

        assert_eq!(
            record.previous_content.as_deref(),
            Some("original_content=1\n")
        );

        // Mutate file
        fs::write(&target_file, "original_content=2\n").unwrap();
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "original_content=2\n");

        // Apply rollback
        let restored = journal.apply_rollback(&record.id).unwrap();
        assert_eq!(restored.id, record.id);
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "original_content=1\n");
    }

    #[test]
    fn test_rollback_deletes_newly_created_file() {
        let tmp = tempdir().unwrap();
        let target_file = tmp.path().join("new_file.conf");

        let journal = RollbackJournal::new(tmp.path()).unwrap();

        // Snapshot a file that does not yet exist
        let record = journal
            .snapshot_file(&target_file, None, "Create drop-in")
            .unwrap();
        assert!(record.previous_content.is_none());

        // Create file
        fs::write(&target_file, "new_content\n").unwrap();
        assert!(target_file.exists());

        // Rollback should delete it
        journal.apply_rollback(&record.id).unwrap();
        assert!(!target_file.exists());
    }

    #[test]
    fn test_list_records_filtering() {
        let tmp = tempdir().unwrap();
        let journal = RollbackJournal::new(tmp.path()).unwrap();

        let _rec1 = journal.snapshot_file("/tmp/a", None, "Action A").unwrap();
        let rec2 = journal.snapshot_file("/tmp/b", None, "Action B").unwrap();

        let all = journal.list_records(0, 10).unwrap();
        assert_eq!(all.len(), 2);

        let limited = journal.list_records(0, 1).unwrap();
        assert_eq!(limited.len(), 1);

        let future = journal.list_records(rec2.timestamp_us + 1_000_000, 10).unwrap();
        assert!(future.is_empty());
    }
}

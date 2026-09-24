//! Edge tests for corrupt, truncated, or binary rollback journal logs.

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use tempfile::tempdir;
    use toold_core::journal::RollbackJournal;

    #[test]
    fn test_corrupt_journal_lines_ignored() {
        let tmp = tempdir().unwrap();
        let journal = RollbackJournal::new(tmp.path()).unwrap();

        let rec1 = journal
            .snapshot_file("/tmp/valid_1.conf", None, "Valid 1")
            .unwrap();

        // Inject garbage lines into the jsonl file
        let log_file = tmp.path().join("rollback.jsonl");
        {
            let mut f = OpenOptions::new().append(true).open(&log_file).unwrap();
            f.write_all(b"{\"invalid\": json without closure\n").unwrap();
            f.write_all(b"\x00\xFF\xAA non utf8 binary garbage\n").unwrap();
            f.write_all(b"\n   \n\t\n").unwrap();
        }

        let rec2 = journal
            .snapshot_file("/tmp/valid_2.conf", None, "Valid 2")
            .unwrap();

        // Query should return both valid records, gracefully skipping garbage
        let records = journal.list_records(0, 10).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, rec1.id);
        assert_eq!(records[1].id, rec2.id);
    }
}

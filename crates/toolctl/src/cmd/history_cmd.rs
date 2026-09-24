//! Handler for listing historical rollback snapshots.

use crate::client::TooldClient;
use anyhow::Result;
use serde_json::json;

/// Executes the `history` command.
pub async fn exec_history(
    client: &TooldClient,
    since_seconds: u64,
    limit: usize,
    as_json: bool,
) -> Result<()> {
    let params = json!({
        "since_seconds": since_seconds,
        "limit": limit,
    });

    let reply = client
        .call("io.syntrop.Tool1.ListRollbacks", Some(params))
        .await?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&reply)?);
        return Ok(());
    }

    let records = reply
        .get("records")
        .and_then(|r| r.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    if records.is_empty() {
        println!("No rollback snapshots recorded in the specified timeframe.");
        return Ok(());
    }

    println!("{:<28} {:<24} {}", "SNAPSHOT ID", "TARGET UNIT", "SUMMARY");
    println!("{}", "-".repeat(80));

    for rec in records {
        let id = rec.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let unit = rec.get("target_unit").and_then(|v| v.as_str()).unwrap_or("-");
        let summary = rec.get("summary").and_then(|v| v.as_str()).unwrap_or("-");

        println!("{:<28} {:<24} {}", id, unit, summary);
    }

    Ok(())
}

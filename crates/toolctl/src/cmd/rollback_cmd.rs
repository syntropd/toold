//! Handler for rolling back actions.

use crate::client::TooldClient;
use anyhow::Result;
use serde_json::json;

/// Executes the `rollback` command.
pub async fn exec_rollback(client: &TooldClient, rollback_id: &str, as_json: bool) -> Result<()> {
    let params = json!({
        "rollback_id": rollback_id,
    });

    let reply = client.call("io.syntrop.Tool1.Rollback", Some(params)).await?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&reply)?);
        return Ok(());
    }

    if let Some(restored) = reply.get("restored") {
        let id = restored.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let path = restored
            .get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let summary = restored
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        println!("Successfully applied rollback: {}", id);
        println!("  Target Path: {}", path);
        println!("  Summary:     {}", summary);
    }

    Ok(())
}

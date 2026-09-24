//! Handler for listing registered tools.

use crate::client::TooldClient;
use anyhow::Result;

/// Executes the `list` command.
pub async fn exec_list(client: &TooldClient, as_json: bool) -> Result<()> {
    let res = client.call("io.syntrop.Tool1.ListTools", None).await?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    let tools = res
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    if tools.is_empty() {
        println!("No tools currently registered with toold.");
        return Ok(());
    }

    println!("{:<18} {:<24} {:<10} {}", "TOOL NAME", "MODE", "TIMEOUT", "DESCRIPTION");
    println!("{}", "-".repeat(80));

    for tool in tools {
        let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let mode = tool.get("mode").and_then(|v| v.as_str()).unwrap_or("-");
        let timeout = tool
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .map(|t| format!("{}ms", t))
            .unwrap_or_else(|| "-".into());
        let desc = tool.get("description").and_then(|v| v.as_str()).unwrap_or("-");

        println!("{:<18} {:<24} {:<10} {}", name, mode, timeout, desc);
    }

    Ok(())
}

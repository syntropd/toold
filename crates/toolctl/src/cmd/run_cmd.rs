//! Handler for invoking sandboxed tools.

use crate::client::TooldClient;
use anyhow::Result;
use serde_json::json;

/// Executes the `run` command.
pub async fn exec_run(
    client: &TooldClient,
    tool_name: &str,
    args: &[String],
    target_unit: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let mut params = json!({
        "name": tool_name,
        "args": args,
    });

    if let Some(u) = target_unit {
        params["target_unit"] = json!(u);
    }

    let reply = client
        .call("io.syntrop.Tool1.ExecuteTool", Some(params))
        .await?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&reply)?);
        return Ok(());
    }

    if let Some(rollback_id) = reply.get("rollback_id").and_then(|v| v.as_str()) {
        println!("Pre-execution snapshot recorded: {}", rollback_id);
    }

    if let Some(res) = reply.get("result") {
        let code = res.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
        let duration = res.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);

        if let Some(stdout) = res.get("stdout").and_then(|v| v.as_str()) {
            if !stdout.is_empty() {
                println!("{}", stdout);
            }
        }

        if let Some(stderr) = res.get("stderr").and_then(|v| v.as_str()) {
            if !stderr.is_empty() {
                eprintln!("{}", stderr);
            }
        }

        if code != 0 {
            std::process::exit(code as i32);
        } else {
            eprintln!("\n[Execution completed in {}ms, exit code: 0]", duration);
        }
    }

    Ok(())
}

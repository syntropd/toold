//! Standard org.varlink.service introspection for toold.

use super::protocol::VarlinkReply;
use serde_json::json;

/// Varlink interface definition text for io.syntrop.Tool1.
pub const IO_SYNTROP_TOOL1_INTERFACE: &str = r#"
interface io.syntrop.Tool1

type ToolInfo (
  name: string,
  description: string,
  mode: string,
  timeout_ms: int
)

type ExecutionResult (
  command: string,
  exit_code: int,
  stdout: string,
  stderr: string,
  duration_ms: int
)

type RollbackRecord (
  id: string,
  timestamp_us: int,
  target_path: ?string,
  target_unit: ?string,
  summary: string
)

method ListTools() -> (tools: []ToolInfo)
method ExecuteTool(name: string, args: []string, target_unit: ?string) -> (result: ExecutionResult, rollback_id: ?string)
method Rollback(rollback_id: string) -> (restored: RollbackRecord)
method ListRollbacks(since_seconds: int, limit: int) -> (records: []RollbackRecord)

error ToolNotFound(name: string)
error ExecutionFailed(reason: string)
error PermissionDenied(reason: string)
error Timeout(limit_ms: int)
error InvalidParameter(parameter: string)
"#;

/// Handles standard org.varlink.service method dispatches.
pub fn handle_service_call(method: &str, params: Option<&serde_json::Value>) -> Option<VarlinkReply> {
    match method {
        "org.varlink.service.GetInfo" => Some(VarlinkReply::ok(json!({
            "vendor": "Syntropd Project",
            "product": "toold",
            "version": env!("CARGO_PKG_VERSION"),
            "url": "https://github.com/syntropd/toold",
            "interfaces": [
                "org.varlink.service",
                "io.syntrop.Tool1"
            ]
        }))),
        "org.varlink.service.GetInterfaceDescription" => {
            let iface = params
                .and_then(|p| p.get("interface"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match iface {
                "io.syntrop.Tool1" => Some(VarlinkReply::ok(json!({
                    "description": IO_SYNTROP_TOOL1_INTERFACE.trim()
                }))),
                _ => Some(VarlinkReply::err(
                    "org.varlink.service.InterfaceNotFound",
                    Some(json!({ "interface": iface })),
                )),
            }
        }
        _ => None,
    }
}

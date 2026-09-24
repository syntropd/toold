//! Implementation of io.syntrop.Tool1 Varlink interface.

use super::protocol::VarlinkReply;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use toold_core::journal::RollbackJournal;
use toold_core::policy::{ExecutionMode, ToolRegistry};
use toold_core::sandbox::execute_tool;

/// Shared state required for Tool1 method dispatches.
#[derive(Clone)]
pub struct Tool1Handler {
    registry: Arc<ToolRegistry>,
    journal: Arc<RollbackJournal>,
}

impl Tool1Handler {
    /// Constructs a new Tool1Handler.
    pub fn new(registry: Arc<ToolRegistry>, journal: Arc<RollbackJournal>) -> Self {
        Self { registry, journal }
    }

    /// Dispatches incoming io.syntrop.Tool1 method calls.
    pub async fn handle_call(&self, method: &str, params: Option<&Value>) -> Option<VarlinkReply> {
        match method {
            "io.syntrop.Tool1.ListTools" => Some(self.handle_list_tools()),
            "io.syntrop.Tool1.ExecuteTool" => Some(self.handle_execute_tool(params).await),
            "io.syntrop.Tool1.Rollback" => Some(self.handle_rollback(params)),
            "io.syntrop.Tool1.ListRollbacks" => Some(self.handle_list_rollbacks(params)),
            _ => None,
        }
    }

    fn handle_list_tools(&self) -> VarlinkReply {
        let tools: Vec<Value> = self
            .registry
            .list()
            .into_iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "mode": format!("{:?}", t.mode),
                    "timeout_ms": t.timeout_ms,
                })
            })
            .collect();

        VarlinkReply::ok(json!({ "tools": tools }))
    }

    async fn handle_execute_tool(&self, params: Option<&Value>) -> VarlinkReply {
        let params = match params {
            Some(p) => p,
            None => {
                return VarlinkReply::err(
                    "io.syntrop.Tool1.InvalidParameter",
                    Some(json!({ "parameter": "parameters" })),
                )
            }
        };

        let name = match params.get("name").and_then(|n| n.as_str()) {
            Some(n) => n,
            None => {
                return VarlinkReply::err(
                    "io.syntrop.Tool1.InvalidParameter",
                    Some(json!({ "parameter": "name" })),
                )
            }
        };

        let tool = match self.registry.get(name) {
            Some(t) => t,
            None => {
                return VarlinkReply::err(
                    "io.syntrop.Tool1.ToolNotFound",
                    Some(json!({ "name": name })),
                )
            }
        };

        let empty_vec = vec![];
        let args: Vec<String> = params
            .get("args")
            .and_then(|a| a.as_array())
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect();

        let target_unit = params.get("target_unit").and_then(|u| u.as_str());

        let mut rollback_id = None;
        if tool.mode == ExecutionMode::RemediateWithRollback {
            if let Some(target_file) = tool.write_paths.first() {
                if let Ok(rec) = self.journal.snapshot_file(
                    target_file,
                    target_unit,
                    &format!("Pre-execution snapshot for {}", tool.name),
                ) {
                    rollback_id = Some(rec.id);
                }
            }
        }

        match execute_tool(&tool, &args, None).await {
            Ok(result) => VarlinkReply::ok(json!({
                "result": result,
                "rollback_id": rollback_id,
            })),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Tool1.ExecutionFailed",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }

    fn handle_rollback(&self, params: Option<&Value>) -> VarlinkReply {
        let rollback_id = match params.and_then(|p| p.get("rollback_id")).and_then(|id| id.as_str()) {
            Some(id) => id,
            None => {
                return VarlinkReply::err(
                    "io.syntrop.Tool1.InvalidParameter",
                    Some(json!({ "parameter": "rollback_id" })),
                )
            }
        };

        match self.journal.apply_rollback(rollback_id) {
            Ok(rec) => VarlinkReply::ok(json!({ "restored": rec })),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Tool1.ExecutionFailed",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }

    fn handle_list_rollbacks(&self, params: Option<&Value>) -> VarlinkReply {
        let since_seconds = params
            .and_then(|p| p.get("since_seconds"))
            .and_then(|s| s.as_u64())
            .unwrap_or(86400);

        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|l| l.as_u64())
            .unwrap_or(50) as usize;

        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        let since_us = now_us.saturating_sub(since_seconds * 1_000_000);

        match self.journal.list_records(since_us, limit) {
            Ok(records) => VarlinkReply::ok(json!({ "records": records })),
            Err(e) => VarlinkReply::err(
                "io.syntrop.Tool1.ExecutionFailed",
                Some(json!({ "reason": e.to_string() })),
            ),
        }
    }
}

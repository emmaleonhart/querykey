//! Minimal MCP (Model Context Protocol) server endpoint.
//!
//! Per the vision, QueryKey "exposes itself as an MCP server so any
//! agent (Claude, Gemma, …) can attend over the graph and act on the
//! files." This is the day-one infrastructure: a model-agnostic way in.
//!
//! Scope: JSON-RPC 2.0 over HTTP POST at `/mcp` implementing the core
//! handshake (`initialize`), `tools/list`, and `tools/call` for a small
//! set of read tools over the derived graph. Deliberately minimal and
//! dependency-free (no `rmcp` yet).
//!
//! TODO(port): stdio + HTTP/SSE transports, the full capability set,
//! resources/prompts, strict notification (no-response) semantics, and
//! write tools governed by `agents.md`.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::api::AppState;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// axum handler: POST /mcp — one JSON-RPC request, one response.
pub async fn mcp_handler(
    State(s): State<Arc<AppState>>,
    Json(req): Json<Value>,
) -> Json<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    // Notifications (no id) get an empty ack; simple clients tolerate it.
    if method.starts_with("notifications/") {
        return Json(json!({ "jsonrpc": "2.0" }));
    }

    match method {
        "initialize" => ok(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "querykey-server", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({ "tools": tool_specs() })),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_tool(&s, name, &args).await {
                Ok(text) => ok(
                    id,
                    json!({ "content": [ { "type": "text", "text": text } ], "isError": false }),
                ),
                Err(e) => ok(
                    id,
                    json!({ "content": [ { "type": "text", "text": e.to_string() } ], "isError": true }),
                ),
            }
        }
        _ => Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("method not found: {method}") }
        })),
    }
}

fn ok(id: Value, result: Value) -> Json<Value> {
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "query_graph",
            "description": "Run a SPARQL SELECT over the derived QueryKey graph (Loca).",
            "inputSchema": {
                "type": "object",
                "properties": { "sparql": { "type": "string" } },
                "required": ["sparql"]
            }
        },
        {
            "name": "list_persons",
            "description": "List all people known to the graph.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "server_health",
            "description": "Graph backend name and reachability.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

async fn call_tool(s: &AppState, name: &str, args: &Value) -> anyhow::Result<String> {
    match name {
        "query_graph" => {
            let sparql = args
                .get("sparql")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'sparql' argument"))?;
            let r = s.graph.query(sparql).await?;
            Ok(serde_json::to_string_pretty(&r)?)
        }
        "list_persons" => {
            let p = s.graph.get_all_persons().await?;
            Ok(serde_json::to_string_pretty(&p)?)
        }
        "server_health" => Ok(json!({
            "backend": s.graph.backend(),
            "ok": s.graph.ping().await.is_ok()
        })
        .to_string()),
        other => Err(anyhow::anyhow!("unknown tool: {other}")),
    }
}

use super::tools::ToolContext;
use serde_json::{json, Value};

pub fn handle_rpc(ctx: &ToolContext, body: &Value) -> Value {
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = body.get("params").cloned().unwrap_or(json!({}));

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "sqffy", "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" | "initialized" => {
            return json!({}); // notification — no response body needed by some clients
        }
        "tools/list" => Ok(json!({ "tools": ToolContext::catalog() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match ctx.call(name, &args) {
                Ok(value) => Ok(json!({
                    "content": [{ "type": "text", "text": value.to_string() }],
                    "structuredContent": value,
                    "isError": false
                })),
                Err(err) => Ok(json!({
                    "content": [{ "type": "text", "text": err.to_string() }],
                    "isError": true
                })),
            }
        }
        "ping" => Ok(json!({})),
        other => Err(format!("method not found: {other}")),
    };

    match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": e }
        }),
    }
}

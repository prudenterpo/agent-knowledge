//! Line-delimited JSON-RPC MCP adapter over the validated application core.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::{
    CreateRecord, Error, HandoffStore, KnowledgeStore, RecordCategory, UpdateRecord, WriteHandoff,
    render_briefing,
};

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// Serve one request at a time using stdin and stdout-compatible streams.
pub fn serve<R: BufRead, W: Write>(
    store: &KnowledgeStore,
    input: R,
    mut output: W,
) -> Result<(), Error> {
    for line in input.lines() {
        let line = line.map_err(|error| Error::Protocol(format!("read MCP input: {error}")))?;
        let response = if line.len() > MAX_MESSAGE_BYTES {
            error_response(Value::Null, "input exceeds the 1 MiB limit")
        } else {
            handle(store, &line)
        };
        serde_json::to_writer(&mut output, &response)
            .map_err(|error| Error::Protocol(format!("write MCP response: {error}")))?;
        output
            .write_all(b"\n")
            .map_err(|error| Error::Protocol(format!("write MCP response: {error}")))?;
        output
            .flush()
            .map_err(|error| Error::Protocol(format!("flush MCP response: {error}")))?;
    }
    Ok(())
}

fn handle(store: &KnowledgeStore, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return error_response(Value::Null, "malformed JSON-RPC message"),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" | "tools/list" => Ok(json!({"tools": tool_names()})),
        "tools/call" => call_tool(store, request.get("params").unwrap_or(&Value::Null)),
        _ => Err(Error::Protocol("unknown MCP method".to_string())),
    };
    match result {
        Ok(value) => json!({"jsonrpc":"2.0", "id":id, "result":value}),
        Err(error) => error_response(id, &error.to_string()),
    }
}

fn call_tool(store: &KnowledgeStore, params: &Value) -> Result<Value, Error> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Protocol("tool name is required".to_string()))?;
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    match name {
        "agent_knowledge_briefing" => {
            Ok(json!({"content": render_briefing(store, &HandoffStore::new(store.project()))?}))
        }
        "agent_knowledge_search" => {
            let query = string(arguments, "query")?;
            let obsolete = arguments
                .get("include_obsolete")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            let result = store.search(query, obsolete, limit)?;
            Ok(
                json!({"records": result.records.iter().map(record_json).collect::<Vec<_>>(), "truncated": result.truncated}),
            )
        }
        "agent_knowledge_get_record" => Ok(record_json(&store.get(string(arguments, "id")?)?)),
        "agent_knowledge_create_record" => {
            let record = store.create(CreateRecord {
                title: string(arguments, "title")?,
                category: category(string(arguments, "category")?)?,
                content: string(arguments, "content")?,
                priority: arguments
                    .get("priority")
                    .and_then(Value::as_u64)
                    .map(|value| value as u8),
                pinned: arguments.get("pinned").and_then(Value::as_bool),
                supersedes: arguments.get("supersedes").and_then(Value::as_str),
            })?;
            Ok(record_json(&record))
        }
        "agent_knowledge_update_record" => {
            let expected = arguments
                .get("expected_revision")
                .and_then(Value::as_u64)
                .ok_or_else(|| Error::Validation("expected_revision is required".to_string()))?;
            let record = store.update(
                string(arguments, "id")?,
                expected,
                UpdateRecord {
                    content: arguments.get("content").and_then(Value::as_str),
                    title: arguments.get("title").and_then(Value::as_str),
                    priority: arguments
                        .get("priority")
                        .and_then(Value::as_u64)
                        .map(|value| value as u8),
                    ..UpdateRecord::default()
                },
            )?;
            Ok(record_json(&record))
        }
        "agent_knowledge_list_handoffs" => Ok(
            json!({"handoffs": HandoffStore::new(store.project()).previous()?.iter().map(|handoff| json!({"id":handoff.id,"revision":handoff.revision,"content":handoff.content})).collect::<Vec<_>>() }),
        ),
        "agent_knowledge_write_handoff" => {
            let expected = arguments.get("expected_revision").and_then(Value::as_u64);
            let handoff = HandoffStore::new(store.project()).write(
                expected,
                WriteHandoff {
                    completed: string(arguments, "completed")?,
                    pending: string(arguments, "pending")?,
                    decisions_or_limitations: string(arguments, "decisions_or_limitations")?,
                    next_step: string(arguments, "next_step")?,
                },
            )?;
            Ok(json!({"id":handoff.id,"revision":handoff.revision}))
        }
        _ => Err(Error::Protocol("unsupported MCP tool".to_string())),
    }
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, Error> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Validation(format!("{field} is required")))
}
fn category(value: &str) -> Result<RecordCategory, Error> {
    match value {
        "decision" => Ok(RecordCategory::Decision),
        "gotcha" => Ok(RecordCategory::Gotcha),
        "procedure" => Ok(RecordCategory::Procedure),
        "note" => Ok(RecordCategory::Note),
        _ => Err(Error::Validation(
            "category must be decision, gotcha, procedure or note".to_string(),
        )),
    }
}
fn tool_names() -> Vec<&'static str> {
    vec![
        "agent_knowledge_briefing",
        "agent_knowledge_search",
        "agent_knowledge_get_record",
        "agent_knowledge_create_record",
        "agent_knowledge_update_record",
        "agent_knowledge_write_handoff",
        "agent_knowledge_list_handoffs",
    ]
}
fn record_json(record: &crate::Record) -> Value {
    json!({"id":record.id,"title":record.title,"category":format!("{:?}",record.category).to_lowercase(),"revision":record.revision,"content":record.content})
}
fn error_response(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32000,"message":message}})
}

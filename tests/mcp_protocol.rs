//! Scenarios from `Feature: MCP protocol`.

use std::fs;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_knowledge::{KnowledgeStore, initialize, serve_mcp, start};

struct Fixture {
    root: PathBuf,
    store: KnowledgeStore,
}
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
impl Fixture {
    fn new() -> Self {
        let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "agent-knowledge-mcp-{}-{number}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let code = root.join("code");
        let memory = root.join("memory");
        fs::create_dir_all(&code).unwrap();
        initialize(&code, &memory).unwrap();
        Self {
            root,
            store: KnowledgeStore::open(start(&code, &memory).unwrap()).unwrap(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn call(store: &KnowledgeStore, input: &str) -> String {
    let mut output = Vec::new();
    serve_mcp(store, BufReader::new(Cursor::new(input)), &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Handshake announces the supported tools
#[test]
fn handshake_announces_the_supported_tools() {
    let fixture = Fixture::new();
    let output = call(
        &fixture.store,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n",
    );
    assert!(output.contains("agent_knowledge_search"));
    assert!(output.contains("agent_knowledge_write_handoff"));
}

/// Tools are announced only for an initialized project
#[test]
fn tools_are_announced_only_for_an_initialized_project() {
    let fixture = Fixture::new();
    let output = call(
        &fixture.store,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
    );
    assert!(output.contains("inputSchema"));
    assert!(output.contains("agent_knowledge_create_record"));
}

/// MCP updates accept every writable record field
#[test]
fn mcp_updates_accept_every_writable_record_field() {
    let fixture = Fixture::new();
    let create = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"agent_knowledge_create_record\",\"arguments\":{\"title\":\"old\",\"category\":\"note\",\"content\":\"old\"}}}\n";
    let output = call(&fixture.store, create);
    let id = serde_json::from_str::<serde_json::Value>(&output).unwrap()["result"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let update = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"agent_knowledge_update_record\",\"arguments\":{{\"id\":\"{id}\",\"expected_revision\":1,\"title\":\"new\",\"content\":\"new\",\"category\":\"decision\",\"priority\":3,\"pinned\":true,\"state\":\"active\"}}}}}}\n"
    );
    assert!(call(&fixture.store, &update).contains("\"revision\":2"));
}

/// MCP priority outside its range is rejected
#[test]
fn mcp_priority_outside_its_range_is_rejected() {
    let fixture = Fixture::new();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"agent_knowledge_create_record\",\"arguments\":{\"title\":\"x\",\"category\":\"note\",\"content\":\"x\",\"priority\":256}}}\n";
    assert!(call(&fixture.store, input).contains("priority"));
}

/// A malformed message returns a structured error
#[test]
fn a_malformed_message_returns_a_structured_error() {
    let fixture = Fixture::new();
    assert!(call(&fixture.store, "{not json}\n").contains("error"));
}

/// End of input shuts down cleanly
#[test]
fn end_of_input_shuts_down_cleanly() {
    let fixture = Fixture::new();
    assert!(serve_mcp(&fixture.store, BufReader::new(Cursor::new("")), Vec::new()).is_ok());
}

/// An oversized input is refused before processing
#[test]
fn an_oversized_input_is_refused_before_processing() {
    let fixture = Fixture::new();
    let input = format!("{}\n", "x".repeat(1024 * 1024 + 1));
    assert!(call(&fixture.store, &input).contains("input exceeds"));
}

/// stdout carries protocol only
#[test]
fn stdout_carries_protocol_only() {
    let fixture = Fixture::new();
    let output = call(
        &fixture.store,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
    );
    assert!(serde_json::from_str::<serde_json::Value>(output.trim()).is_ok());
}

/// No secret or record content leaks into diagnostics
#[test]
fn no_secret_or_record_content_leaks_into_diagnostics() {
    let fixture = Fixture::new();
    let secret = "secret-record-content";
    let input = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"agent_knowledge_create_record\",\"arguments\":{{\"title\":\"\",\"category\":\"note\",\"content\":\"{secret}\"}}}}}}\n"
    );
    assert!(!call(&fixture.store, &input).contains(secret));
}

/// Requests are served one at a time
#[test]
fn requests_are_served_one_at_a_time() {
    let fixture = Fixture::new();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n";
    let output = call(&fixture.store, input);
    assert_eq!(output.lines().count(), 2);
    assert!(output.contains("\"id\":1"));
    assert!(output.contains("\"id\":2"));
}

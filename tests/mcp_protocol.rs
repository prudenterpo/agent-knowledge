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

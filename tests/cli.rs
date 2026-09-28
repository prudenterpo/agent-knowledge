//! Scenarios from `Feature: CLI`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    code: PathBuf,
    memory: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("agent-knowledge-cli-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let code = root.join("code");
        let memory = root.join("memory");
        fs::create_dir_all(&code).unwrap();
        Self { root, code, memory }
    }
    fn run(&self, arguments: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_agent-knowledge"))
            .current_dir(&self.code)
            .args(arguments)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Success exits zero
#[test]
fn success_exits_zero() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .run(&["init", "--memory", fixture.memory.to_str().unwrap()])
            .status
            .success()
    );
}

/// Invalid input exits with its own code
#[test]
fn invalid_input_exits_with_its_own_code() {
    let fixture = Fixture::new();
    let output = fixture.run(&["init", "--memory", fixture.memory.to_str().unwrap()]);
    assert!(output.status.success());
    assert_eq!(
        fixture
            .run(&["create", "--memory", fixture.memory.to_str().unwrap()])
            .status
            .code(),
        Some(64)
    );
}

/// Rebuilding the index is available as its own operation
#[test]
fn rebuilding_the_index_is_available_as_its_own_operation() {
    let fixture = Fixture::new();
    assert!(
        fixture
            .run(&["init", "--memory", fixture.memory.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        fixture
            .run(&[
                "rebuild-index",
                "--memory",
                fixture.memory.to_str().unwrap()
            ])
            .status
            .success()
    );
}

//! Scenarios from `Feature: Project identity` in `spec/SPEC.md`.
//!
//! Each test's doc line is the scenario name.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process;

use agent_knowledge::{Error, MANIFEST_FILE_NAME, ProjectId, initialize, start};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let mut bytes = [0_u8; 8];
        let mut source = fs::File::open("/dev/urandom").unwrap();
        std::io::Read::read_exact(&mut source, &mut bytes).unwrap();
        let name: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let path = std::env::temp_dir().join(format!("agent-knowledge-{}-{name}", process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn directory(parent: &Path, name: &str) -> PathBuf {
    let path = parent.join(name);
    fs::create_dir(&path).unwrap();
    path
}

fn manifest_text(code_repository: &Path) -> String {
    fs::read_to_string(code_repository.join(MANIFEST_FILE_NAME)).unwrap()
}

fn field<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key} = ")))
        .unwrap()
}

/// Initializing a project writes an immutable manifest
#[test]
fn initializing_a_project_writes_an_immutable_manifest() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");

    let id = initialize(&code, &memory).unwrap();

    assert_eq!(MANIFEST_FILE_NAME, ".agent-knowledge.toml");
    let text = manifest_text(&code);
    assert_eq!(field(&text, "id"), format!("\"{id}\""));
    assert_eq!(field(&text, "version"), "1");
    assert_eq!(
        field(&text, "created"),
        field(&text, "updated"),
        "a new manifest records one creation time"
    );
    assert!(text.ends_with('\n'));
    assert!(!code.join(format!("{MANIFEST_FILE_NAME}.partial")).exists());

    let project_directory = memory.join(id.as_str());
    assert!(project_directory.is_dir());
    let mode = fs::metadata(&project_directory)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);

    let resolved = start(&code, &memory).unwrap();
    assert_eq!(resolved.id(), &id);
    assert_eq!(resolved.created(), resolved.updated());
    assert_eq!(resolved.search().unwrap(), Vec::<String>::new());
}

/// Initializing twice keeps the original id
#[test]
fn initializing_twice_keeps_the_original_id() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");

    let original = initialize(&code, &memory).unwrap();
    let before = fs::read(code.join(MANIFEST_FILE_NAME)).unwrap();
    let again = initialize(&code, &memory).unwrap();

    assert_eq!(again, original);
    assert_eq!(fs::read(code.join(MANIFEST_FILE_NAME)).unwrap(), before);
    let entries = fs::read_dir(&memory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![std::ffi::OsString::from(original.as_str())]);
}

/// An invalid manifest is never overwritten
#[test]
fn an_invalid_manifest_is_never_overwritten() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");
    let manifest = code.join(MANIFEST_FILE_NAME);
    let original = b"this is not a manifest\n";
    fs::write(&manifest, original).unwrap();

    let error = initialize(&code, &memory).unwrap_err();

    assert!(
        error.to_string().contains("malformed"),
        "the error should say what is wrong, got: {error}"
    );
    assert!(matches!(error, Error::InvalidIdentity(_)));
    assert_eq!(fs::read(&manifest).unwrap(), original);
    assert!(
        !memory.exists() || fs::read_dir(&memory).unwrap().next().is_none(),
        "a rejected manifest must not allocate a project directory"
    );
}

/// Identity survives a path change
#[test]
fn identity_survives_a_path_change() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");
    let id = initialize(&code, &memory).unwrap();
    fs::write(
        start(&code, &memory)
            .unwrap()
            .memory_directory()
            .join("decision.md"),
        b"kept",
    )
    .unwrap();

    let moved = root.path().join("moved-code");
    fs::rename(&code, &moved).unwrap();
    let project = start(&moved, &memory).unwrap();

    assert_eq!(project.id().as_str(), id.as_str());
    assert_eq!(project.search().unwrap(), vec!["decision.md".to_string()]);
}

/// Identity survives a fresh checkout that keeps the manifest
#[test]
fn identity_survives_a_fresh_checkout_that_keeps_the_manifest() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");
    let id = initialize(&code, &memory).unwrap();
    fs::write(
        start(&code, &memory)
            .unwrap()
            .memory_directory()
            .join("procedure.md"),
        b"kept",
    )
    .unwrap();

    let checkout = directory(root.path(), "checkout");
    fs::copy(
        code.join(MANIFEST_FILE_NAME),
        checkout.join(MANIFEST_FILE_NAME),
    )
    .unwrap();
    let project = start(&checkout, &memory).unwrap();

    assert_eq!(project.id(), &id);
    assert_eq!(project.search().unwrap(), vec!["procedure.md".to_string()]);
}

/// An uninitialized project exposes no normal tools
#[test]
fn an_uninitialized_project_exposes_no_normal_tools() {
    let root = TempDir::new();
    let code = directory(root.path(), "code");
    let memory = root.path().join("memory");

    let error = start(&code, &memory).unwrap_err();

    assert_eq!(error.to_string(), "project is not initialized");
    assert!(matches!(error, Error::NotInitialized));
    assert!(!code.join(MANIFEST_FILE_NAME).exists());
    assert!(!memory.exists());
    // `search` is a method on the handle `start` refused to return, and it
    // takes no project id, so there is no tool to call against this directory.
    let _bound_search: fn(&agent_knowledge::Project) -> Result<Vec<String>, Error> =
        agent_knowledge::Project::search;
}

/// Two projects cannot read each other
#[test]
fn two_projects_cannot_read_each_other() {
    let root = TempDir::new();
    let memory = root.path().join("memory");
    let first_code = directory(root.path(), "first");
    let second_code = directory(root.path(), "second");
    let first_id = initialize(&first_code, &memory).unwrap();
    let second_id = initialize(&second_code, &memory).unwrap();
    assert_ne!(first_id, second_id);

    let first = start(&first_code, &memory).unwrap();
    let second = start(&second_code, &memory).unwrap();
    fs::write(first.memory_directory().join("only-first.md"), b"one").unwrap();
    fs::write(second.memory_directory().join("only-second.md"), b"two").unwrap();

    assert_eq!(first.search().unwrap(), vec!["only-first.md".to_string()]);
    assert_eq!(second.search().unwrap(), vec!["only-second.md".to_string()]);
    let _no_project_argument: fn(&agent_knowledge::Project) -> Result<Vec<String>, Error> =
        agent_knowledge::Project::search;
    let _id_is_not_a_parameter: fn(&ProjectId) -> &str = ProjectId::as_str;
}

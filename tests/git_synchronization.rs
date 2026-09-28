//! Scenarios from `Feature: Git synchronization`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_knowledge::{Error, GitSync};

struct TempDir(PathBuf);
static NEXT_TEMPORARY: AtomicUsize = AtomicUsize::new(0);
impl TempDir {
    fn new() -> Self {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agent-knowledge-git-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(directory: Option<&Path>, arguments: &[&str]) -> Output {
    let mut command = Command::new("git");
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    command.output().unwrap()
}

fn configure(repository: &Path) {
    assert!(
        git(
            Some(repository),
            &["config", "user.email", "agent@example.test"]
        )
        .status
        .success()
    );
    assert!(
        git(
            Some(repository),
            &["config", "user.name", "Agent Knowledge Test"]
        )
        .status
        .success()
    );
}

/// First run clones the memory repository
#[test]
fn first_run_clones_the_memory_repository() {
    let root = TempDir::new();
    let remote = root.0.join("remote.git");
    assert!(
        git(None, &["init", "--bare", &remote.to_string_lossy()])
            .status
            .success()
    );
    let clone = root.0.join("clone");
    let synchronized = GitSync::clone_if_missing(&remote.to_string_lossy(), &clone).unwrap();
    assert_eq!(synchronized.repository(), clone);
    assert!(clone.join(".git").is_dir());
}

/// A write commits atomically without network
#[test]
fn a_write_commits_atomically_without_network() {
    let root = TempDir::new();
    let repository = root.0.join("memory");
    assert!(
        git(None, &["init", &repository.to_string_lossy()])
            .status
            .success()
    );
    configure(&repository);
    let record = repository.join("record.md");
    fs::write(&record, "authoritative markdown").unwrap();
    let synchronized = GitSync::open(&repository).unwrap();
    synchronized
        .commit(
            &[Path::new("record.md")],
            "feat(record): create 550e8400-e29b-41d4-a716-446655440000",
        )
        .unwrap();
    assert!(
        git(Some(&repository), &["rev-parse", "HEAD"])
            .status
            .success()
    );
    assert!(matches!(
        synchronized.push(),
        Err(Error::Synchronization(_))
    ));
    assert!(
        git(Some(&repository), &["rev-parse", "HEAD"])
            .status
            .success()
    );
}

/// Pull happens before a write
#[test]
fn pull_happens_before_a_write() {
    let root = TempDir::new();
    let remote = root.0.join("remote.git");
    assert!(
        git(None, &["init", "--bare", &remote.to_string_lossy()])
            .status
            .success()
    );
    let first = root.0.join("first");
    let second = root.0.join("second");
    GitSync::clone_if_missing(&remote.to_string_lossy(), &first).unwrap();
    configure(&first);
    fs::write(first.join("record.md"), "first").unwrap();
    let first_sync = GitSync::open(&first).unwrap();
    first_sync
        .commit(&[Path::new("record.md")], "feat(record): create first")
        .unwrap();
    first_sync.push().unwrap();
    GitSync::clone_if_missing(&remote.to_string_lossy(), &second).unwrap();
    configure(&second);
    assert!(GitSync::open(&second).unwrap().pull().is_ok());
    assert_eq!(
        fs::read_to_string(second.join("record.md")).unwrap(),
        "first"
    );
}

/// A rejected push surfaces as a conflict, not a generic error
#[test]
fn a_rejected_push_surfaces_as_a_conflict_not_a_generic_error() {
    let root = TempDir::new();
    let remote = root.0.join("remote.git");
    assert!(
        git(None, &["init", "--bare", &remote.to_string_lossy()])
            .status
            .success()
    );
    let first = root.0.join("first");
    let second = root.0.join("second");
    GitSync::clone_if_missing(&remote.to_string_lossy(), &first).unwrap();
    configure(&first);
    let first_sync = GitSync::open(&first).unwrap();
    fs::write(first.join("a.md"), "first").unwrap();
    first_sync
        .commit(&[Path::new("a.md")], "feat(record): create first")
        .unwrap();
    first_sync.push().unwrap();
    GitSync::clone_if_missing(&remote.to_string_lossy(), &second).unwrap();
    configure(&second);
    let second_sync = GitSync::open(&second).unwrap();
    fs::write(first.join("b.md"), "remote change").unwrap();
    first_sync
        .commit(&[Path::new("b.md")], "feat(record): create remote")
        .unwrap();
    first_sync.push().unwrap();
    fs::write(second.join("c.md"), "local change").unwrap();
    second_sync
        .commit(&[Path::new("c.md")], "feat(record): create local")
        .unwrap();
    assert!(matches!(
        second_sync.push(),
        Err(Error::RevisionConflict(_))
    ));
}

/// History is never rewritten
#[test]
fn history_is_never_rewritten() {
    let root = TempDir::new();
    let repository = root.0.join("memory");
    assert!(
        git(None, &["init", &repository.to_string_lossy()])
            .status
            .success()
    );
    configure(&repository);
    let synchronized = GitSync::open(&repository).unwrap();
    fs::write(repository.join("one.md"), "one").unwrap();
    synchronized
        .commit(&[Path::new("one.md")], "feat(record): create one")
        .unwrap();
    let first = String::from_utf8(git(Some(&repository), &["rev-parse", "HEAD"]).stdout).unwrap();
    fs::write(repository.join("two.md"), "two").unwrap();
    synchronized
        .commit(&[Path::new("two.md")], "feat(record): create two")
        .unwrap();
    assert!(
        git(
            Some(&repository),
            &["merge-base", "--is-ancestor", first.trim(), "HEAD"]
        )
        .status
        .success()
    );
}

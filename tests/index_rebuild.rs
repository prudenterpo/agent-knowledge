//! Scenarios from the index-recovery portion of `Feature: Search`.

use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_knowledge::{CreateRecord, KnowledgeStore, RecordCategory, initialize, start};

struct TempDir(PathBuf);
static NEXT_TEMPORARY: AtomicUsize = AtomicUsize::new(0);

impl TempDir {
    fn new() -> Self {
        let number = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("agent-knowledge-index-{}-{number}", process::id()));
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

fn store(root: &TempDir) -> KnowledgeStore {
    let code = root.0.join("code");
    let memory = root.0.join("memory");
    fs::create_dir(&code).unwrap();
    initialize(&code, &memory).unwrap();
    KnowledgeStore::open(start(&code, &memory).unwrap()).unwrap()
}

/// Index rebuilt from files returns the same results
#[test]
fn index_rebuilt_from_files_returns_the_same_results() {
    let root = TempDir::new();
    let store = store(&root);
    store
        .create(CreateRecord {
            title: "One",
            category: RecordCategory::Decision,
            content: "indexed needle",
            priority: None,
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    store
        .create(CreateRecord {
            title: "Two",
            category: RecordCategory::Gotcha,
            content: "indexed needle again",
            priority: None,
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    let before = store
        .search("indexed needle", false, None)
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    fs::remove_file(store.index_path()).unwrap();
    store.rebuild_index().unwrap();
    let after = store
        .search("indexed needle", false, None)
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    assert_eq!(after, before);
}

/// Corrupted index is rebuilt, not repaired
#[test]
fn corrupted_index_is_rebuilt_not_repaired() {
    let root = TempDir::new();
    let store = store(&root);
    let record = store
        .create(CreateRecord {
            title: "Recover",
            category: RecordCategory::Decision,
            content: "source remains durable",
            priority: None,
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    fs::write(store.index_path(), b"not a sqlite database").unwrap();
    let reopened =
        KnowledgeStore::open(start(&root.0.join("code"), &root.0.join("memory")).unwrap()).unwrap();
    assert_eq!(
        reopened.search("durable", false, None).unwrap().records[0].id,
        record.id
    );
}

//! Scenarios from `Feature: Knowledge records` and the first search scenarios.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_knowledge::{
    CreateRecord, Error, KnowledgeStore, RecordCategory, RecordState, UpdateRecord, initialize,
    start,
};

struct TempDir(PathBuf);
static NEXT_TEMPORARY: AtomicUsize = AtomicUsize::new(0);

impl TempDir {
    fn new() -> Self {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agent-knowledge-records-{}-{sequence}",
            process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
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

fn store(root: &TempDir, name: &str) -> KnowledgeStore {
    let code = root.path().join(name);
    let memory = root.path().join("memory");
    fs::create_dir_all(&code).unwrap();
    initialize(&code, &memory).unwrap();
    KnowledgeStore::open(start(&code, &memory).unwrap()).unwrap()
}

fn create<'a>(store: &KnowledgeStore, title: &'a str, content: &'a str) -> agent_knowledge::Record {
    store
        .create(CreateRecord {
            title,
            category: RecordCategory::Decision,
            content,
            priority: None,
            pinned: None,
            supersedes: None,
        })
        .unwrap()
}

/// Creating a record writes a versioned TOML frontmatter file
#[test]
fn creating_a_record_writes_a_versioned_toml_frontmatter_file() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Use Git", "The source is Markdown.");
    let source = fs::read_to_string(
        store
            .project()
            .memory_directory()
            .join("records")
            .join(format!("{}.md", record.id)),
    )
    .unwrap();
    assert!(source.starts_with("+++\nformat = 1\n"));
    assert!(source.contains(&format!("id = \"{}\"", record.id)));
    assert!(source.contains("category = \"decision\""));
    assert!(source.contains("state = \"active\""));
    assert!(source.ends_with("The source is Markdown."));
}

/// The stored Markdown is not rewritten by the system
#[test]
fn the_stored_markdown_is_not_rewritten_by_the_system() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let body = "- one\n- two\n\n```rust\nlet x = 1;\n```\n\n[[decision]]\n";
    let record = create(&store, "Exact body", body);
    assert_eq!(store.get(&record.id).unwrap().content, body);
}

/// A record without a required field is rejected
#[test]
fn a_record_without_a_required_field_is_rejected() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let error = store
        .create(CreateRecord {
            title: "",
            category: RecordCategory::Note,
            content: "body",
            priority: None,
            pinned: None,
            supersedes: None,
        })
        .unwrap_err();
    assert!(matches!(error, Error::Validation(_)));
    assert!(
        fs::read_dir(store.project().memory_directory().join("records"))
            .unwrap()
            .next()
            .is_none()
    );
}

/// An unknown category is rejected
#[test]
fn an_unknown_category_is_rejected() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let path = store
        .project()
        .memory_directory()
        .join("records")
        .join("550e8400-e29b-41d4-a716-446655440000.md");
    fs::write(&path, "+++\nformat = 1\nid = \"550e8400-e29b-41d4-a716-446655440000\"\ntitle = \"bad\"\ncategory = \"other\"\nstate = \"active\"\npriority = 0\npinned = false\nrevision = 1\ncreated = \"2026-09-28T00:00:00Z\"\nupdated = \"2026-09-28T00:00:00Z\"\n+++\nbody").unwrap();
    assert!(matches!(store.rebuild_index(), Err(Error::Validation(_))));
}

/// Creating a record is searchable before local success
#[test]
fn creating_a_record_is_searchable_before_local_success() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Connection pool", "Pool settings");
    assert_eq!(
        store
            .search("connection pool", false, None)
            .unwrap()
            .records[0]
            .id,
        record.id
    );
}

/// Updating a record requires the expected revision
#[test]
fn updating_a_record_requires_the_expected_revision() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Old", "body");
    let updated = store
        .update(
            &record.id,
            1,
            UpdateRecord {
                title: Some("New"),
                ..UpdateRecord::default()
            },
        )
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.title, "New");
}

/// Updating with a stale revision is a conflict
#[test]
fn updating_with_a_stale_revision_is_a_conflict() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Old", "body");
    let error = store
        .update(
            &record.id,
            0,
            UpdateRecord {
                title: Some("New"),
                ..UpdateRecord::default()
            },
        )
        .unwrap_err();
    assert!(matches!(error, Error::RevisionConflict(_)));
    assert_eq!(store.get(&record.id).unwrap().title, "Old");
}

/// Changing state, priority or supersession bumps the revision
#[test]
fn changing_state_priority_or_supersession_bumps_the_revision() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Old", "body");
    assert_eq!(
        store
            .update(
                &record.id,
                1,
                UpdateRecord {
                    priority: Some(3),
                    ..UpdateRecord::default()
                }
            )
            .unwrap()
            .revision,
        2
    );
}

/// Marking a record obsolete keeps it readable
#[test]
fn marking_a_record_obsolete_keeps_it_readable() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Old", "body");
    let obsolete = store.obsolete(&record.id, 1).unwrap();
    assert_eq!(obsolete.state, RecordState::Obsolete);
    assert_eq!(store.get(&record.id).unwrap().content, "body");
}

/// Superseding links two records of the same project
#[test]
fn superseding_links_two_records_of_the_same_project() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let old = create(&store, "Old", "body");
    let replacement = store
        .create(CreateRecord {
            title: "New",
            category: RecordCategory::Decision,
            content: "new",
            priority: None,
            pinned: None,
            supersedes: Some(&old.id),
        })
        .unwrap();
    let old = store.get(&old.id).unwrap();
    assert_eq!(old.state, RecordState::Obsolete);
    assert_eq!(old.superseded_by.as_deref(), Some(replacement.id.as_str()));
}

/// Superseding across projects is refused
#[test]
fn superseding_across_projects_is_refused() {
    let root = TempDir::new();
    let first = store(&root, "first");
    let second = store(&root, "second");
    let external = create(&second, "External", "body");
    assert!(matches!(
        first.create(CreateRecord {
            title: "New",
            category: RecordCategory::Decision,
            content: "body",
            priority: None,
            pinned: None,
            supersedes: Some(&external.id)
        }),
        Err(Error::NotFound(_))
    ));
}

/// Nothing is ever deleted automatically
#[test]
fn nothing_is_ever_deleted_automatically() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Old", "body");
    store.obsolete(&record.id, 1).unwrap();
    store.rebuild_index().unwrap();
    assert!(
        store
            .project()
            .memory_directory()
            .join("records")
            .join(format!("{}.md", record.id))
            .is_file()
    );
}

/// Plain text input needs no FTS5 syntax
#[test]
fn plain_text_input_needs_no_fts5_syntax() {
    let root = TempDir::new();
    let store = store(&root, "code");
    create(&store, "Connection pool", "single connection pool");
    assert_eq!(
        store
            .search("connection pool", false, None)
            .unwrap()
            .records
            .len(),
        1
    );
}

/// Special syntax characters are treated literally
#[test]
fn special_syntax_characters_are_treated_literally() {
    let root = TempDir::new();
    let store = store(&root, "code");
    create(&store, "Literal NEAR", "quotes and stars");
    assert!(store.search("\"NEAR\" * ( )", false, None).is_ok());
}

/// Obsolete records are excluded unless requested
#[test]
fn obsolete_records_are_excluded_unless_requested() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let old = create(&store, "Decision", "matching body");
    create(&store, "Decision now", "matching body");
    store.obsolete(&old.id, 1).unwrap();
    assert_eq!(
        store.search("matching", false, None).unwrap().records.len(),
        1
    );
    assert_eq!(
        store.search("matching", true, None).unwrap().records.len(),
        2
    );
}

/// An interrupted index update is recovered from Markdown
#[test]
fn an_interrupted_index_update_is_recovered_from_markdown() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let record = create(&store, "Durable", "markdown survives index failure");
    fs::remove_file(store.index_path()).unwrap();
    let reopened = KnowledgeStore::open(
        start(&root.path().join("code"), &root.path().join("memory")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        reopened.search("survives", false, None).unwrap().records[0].id,
        record.id
    );
}

/// A frontmatter format newer than supported is refused per file
#[test]
fn a_frontmatter_format_newer_than_supported_is_refused_per_file() {
    let root = TempDir::new();
    let store = store(&root, "code");
    let valid = create(&store, "Valid", "kept readable");
    let path = store
        .project()
        .memory_directory()
        .join("records")
        .join("550e8400-e29b-41d4-a716-446655440000.md");
    fs::write(path, "+++\nformat = 2\nid = \"550e8400-e29b-41d4-a716-446655440000\"\ntitle = \"future\"\ncategory = \"note\"\nstate = \"active\"\npriority = 0\npinned = false\nrevision = 1\ncreated = \"2026-09-28T00:00:00Z\"\nupdated = \"2026-09-28T00:00:00Z\"\n+++\nfuture").unwrap();
    store.rebuild_index().unwrap();
    assert_eq!(
        store.search("readable", false, None).unwrap().records[0].id,
        valid.id
    );
}

/// Search is limited to the current project
#[test]
fn search_is_limited_to_the_current_project() {
    let root = TempDir::new();
    let first = store(&root, "first");
    let second = store(&root, "second");
    create(&first, "First", "shared words");
    create(&second, "Second", "shared words");
    assert_eq!(
        first.search("shared", false, None).unwrap().records.len(),
        1
    );
}

/// Ties break deterministically
#[test]
fn ties_break_deterministically() {
    let root = TempDir::new();
    let store = store(&root, "code");
    create(&store, "Same", "identical query");
    create(&store, "Same", "identical query");
    let first = store
        .search("identical", false, None)
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let second = store
        .search("identical", false, None)
        .unwrap()
        .records
        .into_iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    assert_eq!(first, second);
}

/// Result set is bounded and reports truncation
#[test]
fn result_set_is_bounded_and_reports_truncation() {
    let root = TempDir::new();
    let store = store(&root, "code");
    for number in 0..21 {
        create(&store, &format!("Record {number}"), "common query");
    }
    let result = store.search("common", false, None).unwrap();
    assert_eq!(result.records.len(), 20);
    assert!(result.truncated);
}

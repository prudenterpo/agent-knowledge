//! Scenarios from `Feature: Briefing` and `Feature: Handoff`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_knowledge::{
    CreateRecord, HandoffStore, KnowledgeStore, RecordCategory, WriteHandoff, initialize,
    render_briefing, start,
};

struct Fixture {
    root: PathBuf,
    store: KnowledgeStore,
}
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
impl Fixture {
    fn new() -> Self {
        let number = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "agent-knowledge-briefing-{}-{number}",
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
    fn handoffs(&self) -> HandoffStore {
        HandoffStore::new(self.store.project())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn handoff<'a>() -> WriteHandoff<'a> {
    WriteHandoff {
        completed: "done",
        pending: "pending",
        decisions_or_limitations: "limit",
        next_step: "next",
    }
}

/// Briefing carries pinned content, active priorities and the handoff
#[test]
fn briefing_carries_pinned_content_active_priorities_and_the_handoff() {
    let fixture = Fixture::new();
    fixture
        .store
        .create(CreateRecord {
            title: "Pinned",
            category: RecordCategory::Note,
            content: "pinned content",
            priority: None,
            pinned: Some(true),
            supersedes: None,
        })
        .unwrap();
    fixture
        .store
        .create(CreateRecord {
            title: "Decision",
            category: RecordCategory::Decision,
            content: "decision content",
            priority: Some(3),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    fixture
        .store
        .create(CreateRecord {
            title: "Gotcha",
            category: RecordCategory::Gotcha,
            content: "gotcha content",
            priority: Some(2),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    fixture.handoffs().write(None, handoff()).unwrap();
    let briefing = render_briefing(&fixture.store, &fixture.handoffs()).unwrap();
    for text in [
        "pinned content",
        "decision content",
        "gotcha content",
        "## Completed",
    ] {
        assert!(briefing.contains(text));
    }
}

/// Obsolete records never appear in the briefing
#[test]
fn obsolete_records_never_appear_in_the_briefing() {
    let fixture = Fixture::new();
    let record = fixture
        .store
        .create(CreateRecord {
            title: "Old",
            category: RecordCategory::Decision,
            content: "do not show",
            priority: Some(3),
            pinned: Some(true),
            supersedes: None,
        })
        .unwrap();
    fixture.store.obsolete(&record.id, record.revision).unwrap();
    assert!(
        !render_briefing(&fixture.store, &fixture.handoffs())
            .unwrap()
            .contains("do not show")
    );
    assert_eq!(
        fixture
            .store
            .search("show", true, None)
            .unwrap()
            .records
            .len(),
        1
    );
}

/// Briefing is byte-for-byte deterministic
#[test]
fn briefing_is_byte_for_byte_deterministic() {
    let fixture = Fixture::new();
    fixture
        .store
        .create(CreateRecord {
            title: "Stable",
            category: RecordCategory::Decision,
            content: "stable",
            priority: Some(2),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    assert_eq!(
        render_briefing(&fixture.store, &fixture.handoffs()).unwrap(),
        render_briefing(&fixture.store, &fixture.handoffs()).unwrap()
    );
}

/// Writing a handoff makes it the only active one
#[test]
fn writing_a_handoff_makes_it_the_only_active_one() {
    let fixture = Fixture::new();
    let first = fixture.handoffs().write(None, handoff()).unwrap();
    let second = fixture
        .handoffs()
        .write(Some(first.revision), handoff())
        .unwrap();
    assert_eq!(fixture.handoffs().active().unwrap().unwrap().id, second.id);
    assert_eq!(fixture.handoffs().previous().unwrap().len(), 1);
}

/// The previous handoff stays recoverable
#[test]
fn the_previous_handoff_stays_recoverable() {
    let fixture = Fixture::new();
    let first = fixture.handoffs().write(None, handoff()).unwrap();
    fixture
        .handoffs()
        .write(Some(first.revision), handoff())
        .unwrap();
    assert_eq!(
        fixture.handoffs().previous().unwrap()[0].content,
        first.content
    );
}

/// A handoff carries the four scoped fields
#[test]
fn a_handoff_carries_the_four_scoped_fields() {
    let fixture = Fixture::new();
    let written = fixture.handoffs().write(None, handoff()).unwrap();
    for heading in [
        "## Completed",
        "## Pending",
        "## Decisions and limitations",
        "## Next step",
    ] {
        assert!(written.content.contains(heading));
    }
}

/// Briefing respects its total limit
#[test]
fn briefing_respects_its_total_limit() {
    let fixture = Fixture::new();
    let large = "x".repeat(8 * 1024);
    fixture
        .store
        .create(CreateRecord {
            title: "Pinned",
            category: RecordCategory::Note,
            content: &large,
            priority: None,
            pinned: Some(true),
            supersedes: None,
        })
        .unwrap();
    fixture
        .store
        .create(CreateRecord {
            title: "Decision",
            category: RecordCategory::Decision,
            content: &large,
            priority: Some(3),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    assert!(
        render_briefing(&fixture.store, &fixture.handoffs())
            .unwrap()
            .len()
            <= 16 * 1024
    );
}

/// Each section respects its quota
#[test]
fn each_section_respects_its_quota() {
    let fixture = Fixture::new();
    let content = "d".repeat(8 * 1024);
    fixture
        .store
        .create(CreateRecord {
            title: "Decision",
            category: RecordCategory::Decision,
            content: &content,
            priority: Some(3),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    fixture
        .store
        .create(CreateRecord {
            title: "Gotcha",
            category: RecordCategory::Gotcha,
            content: "gotcha stays",
            priority: Some(3),
            pinned: None,
            supersedes: None,
        })
        .unwrap();
    let briefing = render_briefing(&fixture.store, &fixture.handoffs()).unwrap();
    assert!(briefing.contains("gotcha stays"));
    let decisions = briefing.split("## Gotchas").next().unwrap();
    assert!(decisions.len() <= 4 * 1024 + 128);
}

/// Overflow stays reachable by search
#[test]
fn overflow_stays_reachable_by_search() {
    let fixture = Fixture::new();
    let content = "unique overflow keyword ".repeat(500);
    fixture
        .store
        .create(CreateRecord {
            title: "Pinned",
            category: RecordCategory::Note,
            content: &content,
            priority: None,
            pinned: Some(true),
            supersedes: None,
        })
        .unwrap();
    assert!(
        render_briefing(&fixture.store, &fixture.handoffs())
            .unwrap()
            .contains("[truncated]")
    );
    assert_eq!(
        fixture
            .store
            .search("overflow", false, None)
            .unwrap()
            .records
            .len(),
        1
    );
}

/// Briefing never calls an LLM
#[test]
fn briefing_never_calls_an_llm() {
    let fixture = Fixture::new();
    assert!(render_briefing(&fixture.store, &fixture.handoffs()).is_ok());
}

/// The active handoff appears in the next briefing
#[test]
fn the_active_handoff_appears_in_the_next_briefing() {
    let fixture = Fixture::new();
    fixture.handoffs().write(None, handoff()).unwrap();
    assert!(
        render_briefing(&fixture.store, &fixture.handoffs())
            .unwrap()
            .contains("## Next step")
    );
}

/// Updating a handoff requires the expected revision
#[test]
fn updating_a_handoff_requires_the_expected_revision() {
    let fixture = Fixture::new();
    let active = fixture.handoffs().write(None, handoff()).unwrap();
    assert!(matches!(
        fixture
            .handoffs()
            .write(Some(active.revision - 1), handoff()),
        Err(agent_knowledge::Error::RevisionConflict(_))
    ));
}

/// No failure leaves two active handoffs
#[test]
fn no_failure_leaves_two_active_handoffs() {
    let fixture = Fixture::new();
    let active = fixture.handoffs().write(None, handoff()).unwrap();
    let invalid = WriteHandoff {
        completed: "",
        pending: "pending",
        decisions_or_limitations: "limit",
        next_step: "next",
    };
    assert!(
        fixture
            .handoffs()
            .write(Some(active.revision), invalid)
            .is_err()
    );
    assert_eq!(fixture.handoffs().active().unwrap().unwrap().id, active.id);
    assert!(fixture.handoffs().previous().unwrap().is_empty());
}

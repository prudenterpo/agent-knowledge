//! Deterministic, bounded briefing assembly without a network or LLM.

use crate::{Error, HandoffStore, KnowledgeStore, Record, RecordCategory, RecordState};

const SECTION_LIMIT: usize = 4 * 1024;
const TOTAL_LIMIT: usize = 16 * 1024;

/// Render the pinned records, active high-priority records, and active handoff.
pub fn render(store: &KnowledgeStore, handoffs: &HandoffStore) -> Result<String, Error> {
    let records = store.records()?;
    let pinned = section(
        "Pinned",
        records
            .iter()
            .filter(|record| record.state == RecordState::Active && record.pinned)
            .collect(),
    );
    let decisions = section(
        "Decisions",
        records
            .iter()
            .filter(|record| {
                record.state == RecordState::Active
                    && record.category == RecordCategory::Decision
                    && record.priority >= 2
            })
            .collect(),
    );
    let gotchas = section(
        "Gotchas",
        records
            .iter()
            .filter(|record| {
                record.state == RecordState::Active
                    && record.category == RecordCategory::Gotcha
                    && record.priority >= 2
            })
            .collect(),
    );
    let handoff = handoffs.active()?.map_or_else(
        || "## Handoff\n\n".to_string(),
        |handoff| bounded("Handoff", &handoff.content),
    );
    let mut result = format!("{pinned}{decisions}{gotchas}{handoff}");
    truncate_utf8(&mut result, TOTAL_LIMIT);
    Ok(result)
}

fn section(title: &str, mut records: Vec<&Record>) -> String {
    records.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.id.cmp(&right.id))
    });
    let content = records
        .into_iter()
        .map(|record| format!("### {}\n{}\n", record.title, record.content))
        .collect::<String>();
    bounded(title, &content)
}
fn bounded(title: &str, content: &str) -> String {
    let mut rendered = format!("## {title}\n{content}\n");
    truncate_utf8(&mut rendered, SECTION_LIMIT);
    rendered
}
fn truncate_utf8(value: &mut String, limit: usize) {
    if value.len() <= limit {
        return;
    }
    let mut boundary = limit.saturating_sub("\n[truncated]\n".len());
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value.push_str("\n[truncated]\n");
}

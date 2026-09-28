//! Non-interactive human CLI adapter.

use std::env;
use std::io::{self, BufReader};
use std::path::PathBuf;

use agent_knowledge::{
    CreateRecord, Error, HandoffStore, KnowledgeStore, RecordCategory, UpdateRecord, WriteHandoff,
    initialize, render_briefing, serve_mcp, start,
};

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(exit_code(&error));
    }
}

fn run(arguments: Vec<String>) -> Result<(), Error> {
    let (command, memory, values) = parse(arguments)?;
    let code = env::current_dir()
        .map_err(|error| Error::Persistence(format!("read current directory: {error}")))?;
    if command == "init" {
        initialize(&code, &memory)?;
        println!("initialized");
        return Ok(());
    }
    let store = KnowledgeStore::open(start(&code, &memory)?)?;
    match command.as_str() {
        "status" => println!("project {}", store.project().id()),
        "search" => for record in store.search(required(&values, 0, "query")?, false, None)?.records { println!("{} {} r{}", record.id, record.title, record.revision); },
        "create" => { let category = category(required(&values, 1, "category")?)?; let record = store.create(CreateRecord { title: required(&values, 0, "title")?, category, content: required(&values, 2, "content")?, priority: None, pinned: None, supersedes: None })?; println!("{} r{}", record.id, record.revision); }
        "update" => { let revision = required(&values, 1, "expected_revision")?.parse().map_err(|_| Error::Validation("expected_revision must be an integer".to_string()))?; let record = store.update(required(&values, 0, "id")?, revision, UpdateRecord { content: Some(required(&values, 2, "content")?), ..UpdateRecord::default() })?; println!("{} r{}", record.id, record.revision); }
        "obsolete" => { let revision = required(&values, 1, "expected_revision")?.parse().map_err(|_| Error::Validation("expected_revision must be an integer".to_string()))?; let record = store.obsolete(required(&values, 0, "id")?, revision)?; println!("{} r{}", record.id, record.revision); }
        "briefing" => print!("{}", render_briefing(&store, &HandoffStore::new(store.project()))?),
        "handoff" => { let handoff = HandoffStore::new(store.project()).write(None, WriteHandoff { completed: required(&values, 0, "completed")?, pending: required(&values, 1, "pending")?, decisions_or_limitations: required(&values, 2, "decisions_or_limitations")?, next_step: required(&values, 3, "next_step")? })?; println!("{} r{}", handoff.id, handoff.revision); }
        "rebuild-index" => { store.rebuild_index()?; println!("index rebuilt"); }
        "sync" => return Err(Error::Synchronization("sync requires a configured Git memory clone; use the core GitSync adapter".to_string())),
        "mcp" => serve_mcp(&store, BufReader::new(io::stdin().lock()), io::stdout().lock())?,
        _ => return Err(Error::Validation("command must be init, status, search, create, update, obsolete, briefing, handoff, sync, rebuild-index or mcp".to_string())),
    }
    Ok(())
}

fn parse(mut values: Vec<String>) -> Result<(String, PathBuf, Vec<String>), Error> {
    let command = values
        .first()
        .cloned()
        .ok_or_else(|| Error::Validation("a command is required".to_string()))?;
    values.remove(0);
    let marker = values
        .iter()
        .position(|value| value == "--memory")
        .ok_or_else(|| Error::Validation("--memory PATH is required".to_string()))?;
    let memory = values
        .get(marker + 1)
        .ok_or_else(|| Error::Validation("--memory requires a path".to_string()))?
        .clone();
    values.drain(marker..=marker + 1);
    Ok((command, PathBuf::from(memory), values))
}
fn required<'a>(values: &'a [String], index: usize, name: &str) -> Result<&'a str, Error> {
    values
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| Error::Validation(format!("{name} is required")))
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
fn exit_code(error: &Error) -> i32 {
    match error {
        Error::Validation(_) | Error::InvalidIdentity(_) | Error::NotInitialized => 64,
        Error::RevisionConflict(_) => 65,
        Error::Synchronization(_) => 69,
        Error::Internal(_) => 70,
        _ => 74,
    }
}

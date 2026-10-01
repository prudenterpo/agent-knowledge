//! Authoritative Markdown records and their disposable SQLite FTS5 index.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Error, GitSync, Project};

const RECORD_FORMAT: u64 = 1;
const MAX_TITLE_BYTES: usize = 512;
const MAX_CONTENT_BYTES: usize = 64 * 1024;
const MAX_QUERY_BYTES: usize = 4 * 1024;
const RESULT_LIMIT: usize = 20;

/// A durable knowledge category.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordCategory {
    /// A decision and the reason behind it.
    Decision,
    /// A failure mode or limitation discovered through work.
    Gotcha,
    /// A repeatable operation that worked.
    Procedure,
    /// Other concise durable context.
    Note,
}

/// Whether a record remains current.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordState {
    /// The record is eligible for normal retrieval.
    Active,
    /// The record remains available but is excluded by default.
    Obsolete,
}

/// Input for a new record.
#[derive(Debug)]
pub struct CreateRecord<'a> {
    /// Human-readable title.
    pub title: &'a str,
    /// Explicit durable category.
    pub category: RecordCategory,
    /// Markdown body, retained without rewriting.
    pub content: &'a str,
    /// Optional priority from 0 through 3.
    pub priority: Option<u8>,
    /// Whether this belongs in the pinned briefing section.
    pub pinned: Option<bool>,
    /// The earlier record made obsolete by this new record.
    pub supersedes: Option<&'a str>,
}

/// A partial mutation guarded by the revision read by its caller.
#[derive(Debug, Default)]
pub struct UpdateRecord<'a> {
    /// Replacement title.
    pub title: Option<&'a str>,
    /// Replacement category.
    pub category: Option<RecordCategory>,
    /// Replacement Markdown body.
    pub content: Option<&'a str>,
    /// Replacement priority.
    pub priority: Option<u8>,
    /// Replacement pinned flag.
    pub pinned: Option<bool>,
    /// Replacement state.
    pub state: Option<RecordState>,
    /// A record this record supersedes.
    pub supersedes: Option<&'a str>,
}

/// A parsed record returned by the application core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Immutable UUID.
    pub id: String,
    /// Record title.
    pub title: String,
    /// Durable category.
    pub category: RecordCategory,
    /// Current validity state.
    pub state: RecordState,
    /// Retrieval adjustment from zero through three.
    pub priority: u8,
    /// Briefing inclusion marker.
    pub pinned: bool,
    /// Monotonically increasing local revision.
    pub revision: u64,
    /// Creation timestamp.
    pub created: String,
    /// Last modification timestamp.
    pub updated: String,
    /// Optional newer record that supersedes this one.
    pub superseded_by: Option<String>,
    /// Exact Markdown bytes after the TOML frontmatter delimiter.
    pub content: String,
}

/// Bounded search response.
#[derive(Debug)]
pub struct SearchResults {
    /// Ordered matching records.
    pub records: Vec<Record>,
    /// Whether further matches existed beyond the configured limit.
    pub truncated: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Frontmatter {
    format: u64,
    id: String,
    title: String,
    category: RecordCategory,
    state: RecordState,
    priority: u8,
    pinned: bool,
    revision: u64,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    superseded_by: Option<String>,
}

/// Bound application service for one project.
pub struct KnowledgeStore {
    project: Project,
    index_path: PathBuf,
    git: Option<GitSync>,
}

impl KnowledgeStore {
    /// Open the derived index for a resolved project.
    pub fn open(project: Project) -> Result<Self, Error> {
        let index_path = project.memory_directory().join(".agent-knowledge.sqlite");
        let repository = project.memory_directory().parent().map(Path::to_path_buf);
        let git = repository
            .as_deref()
            .filter(|path| path.join(".git").is_dir())
            .map(GitSync::open)
            .transpose()?;
        if let Some(git) = &git {
            git.pull()?;
        }
        let store = Self {
            project,
            index_path,
            git,
        };
        store.ensure_records_directory()?;
        store.ensure_index()?;
        Ok(store)
    }

    /// Create a record and update its local search index before success.
    pub fn create(&self, input: CreateRecord<'_>) -> Result<Record, Error> {
        validate_title(input.title)?;
        validate_content(input.content)?;
        let priority = input.priority.unwrap_or(0);
        validate_priority(priority)?;
        let id = Uuid::new_v4().to_string();
        let now = now()?;
        let record = Record {
            id: id.clone(),
            title: input.title.to_string(),
            category: input.category,
            state: RecordState::Active,
            priority,
            pinned: input.pinned.unwrap_or(false),
            revision: 1,
            created: now.clone(),
            updated: now,
            superseded_by: None,
            content: input.content.to_string(),
        };
        self.write_record(&record)?;
        let mut changed = vec![self.record_path(&record.id)];
        if let Some(previous) = input.supersedes {
            let previous = self.superseded_record(&record, previous)?;
            self.write_record(&previous)?;
            changed.push(self.record_path(&previous.id));
            self.index_record(&previous)?;
        }
        if let Err(error) = self.index_record(&record) {
            return Err(Error::Index(format!(
                "record was written but its local index update failed; restart to rebuild: {error}"
            )));
        }
        self.commit_and_push(&changed, "create", &record.id)?;
        Ok(record)
    }

    /// Return one record from this project only.
    pub fn get(&self, id: &str) -> Result<Record, Error> {
        validate_uuid(id)?;
        let path = self.record_path(id);
        if !path.is_file() {
            return Err(Error::NotFound(format!("record {id} was not found")));
        }
        read_record(&path)
    }

    /// Update a record only if its revision equals the value the caller read.
    pub fn update(
        &self,
        id: &str,
        expected_revision: u64,
        change: UpdateRecord<'_>,
    ) -> Result<Record, Error> {
        let mut record = self.get(id)?;
        if record.revision != expected_revision {
            return Err(Error::RevisionConflict(format!(
                "record {id} is at revision {}, not {expected_revision}",
                record.revision
            )));
        }
        if let Some(title) = change.title {
            validate_title(title)?;
            record.title = title.to_string();
        }
        if let Some(category) = change.category {
            record.category = category;
        }
        if let Some(content) = change.content {
            validate_content(content)?;
            record.content = content.to_string();
        }
        if let Some(priority) = change.priority {
            validate_priority(priority)?;
            record.priority = priority;
        }
        if let Some(pinned) = change.pinned {
            record.pinned = pinned;
        }
        if let Some(state) = change.state {
            record.state = state;
        }
        let superseded = change
            .supersedes
            .map(|supersedes| self.superseded_record(&record, supersedes))
            .transpose()?;
        record.revision += 1;
        record.updated = now()?;
        self.write_record(&record)?;
        let mut changed = vec![self.record_path(&record.id)];
        if let Some(previous) = &superseded {
            self.write_record(previous)?;
            changed.push(self.record_path(&previous.id));
            self.index_record(previous)?;
        }
        self.index_record(&record)?;
        self.commit_and_push(&changed, "update", &record.id)?;
        Ok(record)
    }

    /// Mark an existing record obsolete without deleting its source file.
    pub fn obsolete(&self, id: &str, expected_revision: u64) -> Result<Record, Error> {
        self.update(
            id,
            expected_revision,
            UpdateRecord {
                state: Some(RecordState::Obsolete),
                ..UpdateRecord::default()
            },
        )
    }

    /// Find literal words through FTS5, scoped to this store's project.
    pub fn search(
        &self,
        query: &str,
        include_obsolete: bool,
        limit: Option<usize>,
    ) -> Result<SearchResults, Error> {
        if query.is_empty() || query.len() > MAX_QUERY_BYTES {
            return Err(Error::Validation(
                "query must contain at most 4096 UTF-8 bytes".to_string(),
            ));
        }
        let limit = limit.unwrap_or(RESULT_LIMIT).min(RESULT_LIMIT);
        let terms = literal_fts_query(query);
        if terms.is_empty() {
            return Ok(SearchResults {
                records: Vec::new(),
                truncated: false,
            });
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id FROM records_fts WHERE records_fts MATCH ?1 AND (?2 = 1 OR state = 'active') ORDER BY bm25(records_fts) - (priority * 0.001), id LIMIT ?3"
        ).map_err(index_error)?;
        let ids = statement
            .query_map(
                params![terms, i64::from(include_obsolete), (limit + 1) as i64],
                |row| row.get::<_, String>(0),
            )
            .map_err(index_error)?;
        let ids = ids.collect::<Result<Vec<_>, _>>().map_err(index_error)?;
        let truncated = ids.len() > limit;
        let records = ids
            .into_iter()
            .take(limit)
            .map(|id| self.get(&id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SearchResults { records, truncated })
    }

    /// Read every valid record in deterministic file-name order.
    pub fn records(&self) -> Result<Vec<Record>, Error> {
        self.record_paths()?
            .into_iter()
            .map(|path| read_record(&path))
            .collect()
    }

    /// Recreate the entire local index from authoritative Markdown files.
    pub fn rebuild_index(&self) -> Result<(), Error> {
        let temporary = self.index_path.with_extension("sqlite.rebuild");
        let _ = fs::remove_file(&temporary);
        let connection = configure_connection(&temporary)?;
        initialize_schema(&connection)?;
        for path in self.record_paths()? {
            match read_record(&path) {
                Ok(record) => index_into(&connection, &record)?,
                Err(Error::IncompatibleSchema(_)) => continue,
                Err(error) => return Err(error),
            }
        }
        connection
            .close()
            .map_err(|(_, error)| index_error(error))?;
        fs::rename(&temporary, &self.index_path)
            .map_err(|error| Error::Index(format!("replace rebuilt index: {error}")))
    }

    /// Path of the disposable SQLite index, useful for an explicit rebuild command.
    #[must_use]
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// The project this store was bound to at startup.
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    fn ensure_records_directory(&self) -> Result<(), Error> {
        fs::create_dir_all(self.records_directory())
            .map_err(|error| Error::Persistence(format!("create records directory: {error}")))
    }

    fn ensure_index(&self) -> Result<(), Error> {
        if self.index_path.exists() {
            let connection = self.connection();
            if let Ok(connection) = connection {
                let integrity: Result<String, _> =
                    connection.query_row("PRAGMA integrity_check", [], |row| row.get(0));
                if matches!(integrity.as_deref(), Ok("ok")) {
                    return Ok(());
                }
            }
        }
        self.rebuild_index()
    }

    fn connection(&self) -> Result<Connection, Error> {
        let connection = configure_connection(&self.index_path)?;
        initialize_schema(&connection)?;
        Ok(connection)
    }

    fn records_directory(&self) -> PathBuf {
        self.project.memory_directory().join("records")
    }
    fn record_path(&self, id: &str) -> PathBuf {
        self.records_directory().join(format!("{id}.md"))
    }

    fn record_paths(&self) -> Result<Vec<PathBuf>, Error> {
        let mut paths = fs::read_dir(self.records_directory())
            .map_err(|error| Error::Persistence(format!("read records directory: {error}")))?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|type_| type_.is_file())
                    .map(|_| entry.path())
            })
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }

    fn write_record(&self, record: &Record) -> Result<(), Error> {
        write_atomic(&self.record_path(&record.id), &render_record(record)?)
    }
    fn index_record(&self, record: &Record) -> Result<(), Error> {
        let connection = self.connection()?;
        connection
            .execute("DELETE FROM records_fts WHERE id = ?1", params![record.id])
            .map_err(index_error)?;
        index_into(&connection, record)
    }

    fn superseded_record(&self, replacement: &Record, previous_id: &str) -> Result<Record, Error> {
        if replacement.id == previous_id {
            return Err(Error::Validation(
                "a record cannot supersede itself".to_string(),
            ));
        }
        let mut previous = self.get(previous_id)?;
        previous.state = RecordState::Obsolete;
        previous.superseded_by = Some(replacement.id.clone());
        previous.revision += 1;
        previous.updated = now()?;
        Ok(previous)
    }

    fn commit_and_push(&self, paths: &[PathBuf], operation: &str, id: &str) -> Result<(), Error> {
        let Some(git) = &self.git else {
            return Ok(());
        };
        let root = git.repository();
        let paths = paths
            .iter()
            .map(|path| {
                path.strip_prefix(root).map_err(|_| {
                    Error::Internal("record path is outside the memory repository".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        git.commit(&paths, &format!("feat(record): {operation} {id}"))?;
        match git.push() {
            Ok(()) | Err(Error::Synchronization(_)) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

fn configure_connection(path: &Path) -> Result<Connection, Error> {
    let connection = Connection::open(path).map_err(index_error)?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=1000;")
        .map_err(index_error)?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> Result<(), Error> {
    connection.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS records_fts USING fts5(id UNINDEXED, title, content, state UNINDEXED, priority UNINDEXED);").map_err(index_error)
}

fn index_into(connection: &Connection, record: &Record) -> Result<(), Error> {
    connection.execute("INSERT INTO records_fts (id, title, content, state, priority) VALUES (?1, ?2, ?3, ?4, ?5)", params![record.id, record.title, record.content, state_text(record.state), record.priority]).map_err(index_error)?;
    Ok(())
}

fn state_text(state: RecordState) -> &'static str {
    match state {
        RecordState::Active => "active",
        RecordState::Obsolete => "obsolete",
    }
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), Error> {
    let temporary = path.with_extension("md.partial");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| Error::Persistence(format!("create temporary record: {error}")))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| Error::Persistence(format!("write temporary record: {error}")))?;
    file.sync_all()
        .map_err(|error| Error::Persistence(format!("sync temporary record: {error}")))?;
    fs::rename(&temporary, path)
        .map_err(|error| Error::Persistence(format!("rename record: {error}")))?;
    File::open(path.parent().unwrap_or_else(|| Path::new(".")))
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::Persistence(format!("sync records directory: {error}")))
}

fn render_record(record: &Record) -> Result<String, Error> {
    let frontmatter = Frontmatter {
        format: RECORD_FORMAT,
        id: record.id.clone(),
        title: record.title.clone(),
        category: record.category,
        state: record.state,
        priority: record.priority,
        pinned: record.pinned,
        revision: record.revision,
        created: record.created.clone(),
        updated: record.updated.clone(),
        superseded_by: record.superseded_by.clone(),
    };
    let frontmatter = toml::to_string(&frontmatter)
        .map_err(|error| Error::Internal(format!("serialize record frontmatter: {error}")))?;
    Ok(format!("+++\n{frontmatter}+++\n{}", record.content))
}

fn read_record(path: &Path) -> Result<Record, Error> {
    let text = fs::read_to_string(path)
        .map_err(|error| Error::Persistence(format!("read {}: {error}", path.display())))?;
    let Some(remainder) = text.strip_prefix("+++\n") else {
        return Err(Error::Validation(format!(
            "record {} has no TOML frontmatter",
            path.display()
        )));
    };
    let Some((frontmatter, content)) = remainder.split_once("+++\n") else {
        return Err(Error::Validation(format!(
            "record {} has an unterminated frontmatter",
            path.display()
        )));
    };
    let frontmatter: Frontmatter = toml::from_str(frontmatter).map_err(|error| {
        Error::Validation(format!(
            "record {} has invalid TOML frontmatter: {error}",
            path.display()
        ))
    })?;
    if frontmatter.format != RECORD_FORMAT {
        return Err(Error::IncompatibleSchema(format!(
            "record {} has unsupported format {}; upgrade the client",
            path.display(),
            frontmatter.format
        )));
    }
    validate_uuid(&frontmatter.id)?;
    validate_title(&frontmatter.title)?;
    validate_priority(frontmatter.priority)?;
    if frontmatter.revision == 0 {
        return Err(Error::Validation(format!(
            "record {} has revision zero",
            path.display()
        )));
    }
    Ok(Record {
        id: frontmatter.id,
        title: frontmatter.title,
        category: frontmatter.category,
        state: frontmatter.state,
        priority: frontmatter.priority,
        pinned: frontmatter.pinned,
        revision: frontmatter.revision,
        created: frontmatter.created,
        updated: frontmatter.updated,
        superseded_by: frontmatter.superseded_by,
        content: content.to_string(),
    })
}

fn validate_title(title: &str) -> Result<(), Error> {
    if title.is_empty() || title.len() > MAX_TITLE_BYTES {
        Err(Error::Validation(
            "title is required and must contain at most 512 UTF-8 bytes".to_string(),
        ))
    } else {
        Ok(())
    }
}
fn validate_content(content: &str) -> Result<(), Error> {
    if content.len() > MAX_CONTENT_BYTES {
        Err(Error::Validation(
            "content must contain at most 65536 UTF-8 bytes".to_string(),
        ))
    } else {
        Ok(())
    }
}
fn validate_priority(priority: u8) -> Result<(), Error> {
    if priority > 3 {
        Err(Error::Validation(
            "priority must be from 0 through 3".to_string(),
        ))
    } else {
        Ok(())
    }
}
fn validate_uuid(id: &str) -> Result<(), Error> {
    let parsed = Uuid::parse_str(id)
        .map_err(|_| Error::Validation(format!("record id {id} is not a UUID")))?;
    if parsed.get_version_num() != 4 || parsed.to_string() != id {
        return Err(Error::Validation(format!(
            "record id {id} is not a lowercase UUID version 4"
        )));
    }
    Ok(())
}
fn literal_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|term| term.chars().any(char::is_alphanumeric))
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}
fn index_error(error: rusqlite::Error) -> Error {
    Error::Index(format!("local index: {error}"))
}

fn now() -> Result<String, Error> {
    let output = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map_err(|error| Error::Persistence(format!("read UTC clock: {error}")))?;
    String::from_utf8(output.stdout)
        .map_err(|_| Error::Internal("UTC clock returned non-UTF-8 output".to_string()))
        .map(|timestamp| timestamp.trim_end().to_string())
}

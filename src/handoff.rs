//! Durable, single-active project handoffs.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Error, GitSync, Project};

/// Input required for a handoff.
pub struct WriteHandoff<'a> {
    /// Work completed in the previous session.
    pub completed: &'a str,
    /// Work still pending.
    pub pending: &'a str,
    /// Recent decisions or known limitations.
    pub decisions_or_limitations: &'a str,
    /// Concrete next action.
    pub next_step: &'a str,
}

/// A recoverable session handoff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handoff {
    /// Immutable id.
    pub id: String,
    /// Whether this is the current handoff.
    pub active: bool,
    /// Monotonic replacement revision.
    pub revision: u64,
    /// Creation timestamp.
    pub created: String,
    /// Modification timestamp.
    pub updated: String,
    /// Rendered Markdown body with its four sections.
    pub content: String,
}

#[derive(Deserialize, Serialize)]
struct Frontmatter {
    format: u64,
    id: String,
    state: String,
    revision: u64,
    created: String,
    updated: String,
}

/// Handoff persistence bound to one project.
pub struct HandoffStore {
    directory: PathBuf,
    git: Option<GitSync>,
}

impl HandoffStore {
    /// Construct a handoff store for a resolved project.
    #[must_use]
    pub fn new(project: &Project) -> Self {
        let repository = project.memory_directory().parent();
        Self {
            directory: project.memory_directory().join("handoffs"),
            git: repository
                .filter(|path| path.join(".git").is_dir())
                .and_then(|path| GitSync::open(path).ok()),
        }
    }

    /// Read the active handoff when one exists.
    pub fn active(&self) -> Result<Option<Handoff>, Error> {
        if let Some(git) = &self.git {
            git.pull()?;
        }
        let path = self.directory.join("active.md");
        if !path.exists() {
            return Ok(None);
        }
        read_handoff(&path)
    }

    /// Replace the active handoff while keeping the prior file recoverable.
    pub fn write(
        &self,
        expected_revision: Option<u64>,
        input: WriteHandoff<'_>,
    ) -> Result<Handoff, Error> {
        validate_sections(&input)?;
        fs::create_dir_all(&self.directory).map_err(persistence)?;
        let previous = self.active()?;
        if let Some(previous) = &previous
            && expected_revision != Some(previous.revision)
        {
            return Err(Error::RevisionConflict(format!(
                "active handoff is at revision {}, not the expected revision",
                previous.revision
            )));
        }
        let timestamp = now()?;
        let next = Handoff {
            id: Uuid::new_v4().to_string(),
            active: true,
            revision: previous.as_ref().map_or(1, |handoff| handoff.revision + 1),
            created: timestamp.clone(),
            updated: timestamp,
            content: render_body(&input),
        };
        if let Some(mut previous) = previous.clone() {
            previous.active = false;
            previous.updated = next.updated.clone();
            write_atomic(
                &self.directory.join(format!("{}.md", previous.id)),
                &render_handoff(&previous)?,
            )?;
        }
        write_atomic(&self.directory.join("active.md"), &render_handoff(&next)?)?;
        self.commit_and_push(&next, previous.as_ref())?;
        Ok(next)
    }

    /// List historical handoffs in deterministic most-recent-first order.
    pub fn previous(&self) -> Result<Vec<Handoff>, Error> {
        let mut handoffs = fs::read_dir(&self.directory)
            .map_err(persistence)?
            .filter_map(Result::ok)
            .filter_map(|entry| (entry.file_name() != "active.md").then_some(entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .map(|path| read_handoff(&path))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        handoffs.sort_by(|left, right| {
            right
                .updated
                .cmp(&left.updated)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(handoffs)
    }
}

impl HandoffStore {
    fn commit_and_push(&self, next: &Handoff, previous: Option<&Handoff>) -> Result<(), Error> {
        let Some(git) = &self.git else {
            return Ok(());
        };
        let root = git.repository();
        let mut paths = vec![self.directory.join("active.md")];
        if let Some(previous) = previous {
            paths.push(self.directory.join(format!("{}.md", previous.id)));
        }
        let paths = paths
            .iter()
            .map(|path| {
                path.strip_prefix(root).map_err(|_| {
                    Error::Internal("handoff path is outside the memory repository".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        git.commit(&paths, &format!("feat(handoff): write {}", next.id))?;
        match git.push() {
            Ok(()) | Err(Error::Synchronization(_)) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

fn validate_sections(input: &WriteHandoff<'_>) -> Result<(), Error> {
    for (name, section) in [
        ("completed", input.completed),
        ("pending", input.pending),
        ("decisions_or_limitations", input.decisions_or_limitations),
        ("next_step", input.next_step),
    ] {
        if section.is_empty() || section.len() > 16 * 1024 {
            return Err(Error::Validation(format!(
                "handoff {name} is required and must contain at most 16384 UTF-8 bytes"
            )));
        }
    }
    Ok(())
}
fn render_body(input: &WriteHandoff<'_>) -> String {
    format!(
        "## Completed\n{}\n\n## Pending\n{}\n\n## Decisions and limitations\n{}\n\n## Next step\n{}\n",
        input.completed, input.pending, input.decisions_or_limitations, input.next_step
    )
}
fn render_handoff(handoff: &Handoff) -> Result<String, Error> {
    let frontmatter = toml::to_string(&Frontmatter {
        format: 1,
        id: handoff.id.clone(),
        state: if handoff.active {
            "active".to_string()
        } else {
            "inactive".to_string()
        },
        revision: handoff.revision,
        created: handoff.created.clone(),
        updated: handoff.updated.clone(),
    })
    .map_err(|error| Error::Internal(format!("serialize handoff: {error}")))?;
    Ok(format!("+++\n{frontmatter}+++\n{}", handoff.content))
}
fn read_handoff(path: &Path) -> Result<Option<Handoff>, Error> {
    let text = fs::read_to_string(path).map_err(persistence)?;
    let Some(rest) = text.strip_prefix("+++\n") else {
        return Err(Error::Validation(
            "handoff has no TOML frontmatter".to_string(),
        ));
    };
    let Some((frontmatter, content)) = rest.split_once("+++\n") else {
        return Err(Error::Validation(
            "handoff has unterminated TOML frontmatter".to_string(),
        ));
    };
    let frontmatter: Frontmatter = toml::from_str(frontmatter)
        .map_err(|error| Error::Validation(format!("handoff frontmatter is invalid: {error}")))?;
    if frontmatter.format != 1 {
        return Err(Error::IncompatibleSchema(format!(
            "handoff has unsupported format {}",
            frontmatter.format
        )));
    }
    Ok(Some(Handoff {
        id: frontmatter.id,
        active: frontmatter.state == "active",
        revision: frontmatter.revision,
        created: frontmatter.created,
        updated: frontmatter.updated,
        content: content.to_string(),
    }))
}
fn write_atomic(path: &Path, content: &str) -> Result<(), Error> {
    let temporary = path.with_extension("md.partial");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(persistence)?;
    file.write_all(content.as_bytes()).map_err(persistence)?;
    file.sync_all().map_err(persistence)?;
    fs::rename(temporary, path).map_err(persistence)
}
fn persistence(error: std::io::Error) -> Error {
    Error::Persistence(format!("handoff persistence: {error}"))
}
fn now() -> Result<String, Error> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Internal("system clock is before Unix epoch".to_string()))?
        .as_secs();
    let output = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map_err(persistence)?;
    if !output.status.success() || seconds == 0 {
        return Err(Error::Internal("could not read UTC clock".to_string()));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| Error::Internal("UTC clock returned invalid text".to_string()))
        .map(|value| value.trim().to_string())
}

//! Operational memory for a code repository.
//!
//! Knowledge lives as Markdown files in a dedicated Git repository; a local
//! SQLite + FTS5 index is derived from those files and is always disposable.
//! Agents reach this crate through an MCP adapter over stdio; a human reaches
//! it through a CLI. There is no server and no always-on process.
//!
//! Project identity is implemented. [`initialize`] writes
//! `.agent-knowledge.toml` in the code repository and a directory named after
//! the project id in the memory repository. [`start`] resolves that identity
//! from a working directory and refuses a directory that has no manifest.
//! Records, the text index, Git sync, briefing, the CLI and the MCP adapter
//! are not implemented yet.
//!
//! Behavior is specified in `spec/SPEC.md`. Every scenario there is meant to
//! become a test whose name matches the scenario name exactly.

mod error;
mod identity;

pub use error::Error;
pub use identity::{FORMAT_VERSION, MANIFEST_FILE_NAME, Project, ProjectId, initialize, start};

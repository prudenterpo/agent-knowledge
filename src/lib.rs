//! Operational memory for a code repository.
//!
//! Knowledge lives as Markdown files in a dedicated Git repository; a local
//! SQLite + FTS5 index is derived from those files and is always disposable.
//! Agents reach this crate through an MCP adapter over stdio; a human reaches
//! it through a CLI. There is no server and no always-on process.
//!
//! Behavior is specified in `spec/SPEC.md`. Every scenario there is meant to
//! become a test whose name matches the scenario name exactly.
//!
//! This crate is a foundation only: no behavior is implemented yet.

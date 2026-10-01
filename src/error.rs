//! Failures the identity core reports to a caller.
//!
//! Adapters (CLI, MCP) do not exist yet. Messages are already specific enough
//! to show to a person: a missing manifest is distinct from a malformed one,
//! and a malformed manifest is never described as a successful write.

use std::fmt;

/// A failure while resolving or creating a project identity.
#[derive(Debug)]
pub enum Error {
    /// The working directory has no identity manifest.
    NotInitialized,
    /// The manifest exists but this client cannot use it. The file is left as it was.
    InvalidIdentity(String),
    /// A local filesystem operation failed.
    Persistence(String),
    /// Caller-supplied data violates the frozen v1 contract.
    Validation(String),
    /// A record has changed since the caller read the revision it names.
    RevisionConflict(String),
    /// A record id does not exist in the bound project.
    NotFound(String),
    /// A file uses a record schema this client cannot safely read.
    IncompatibleSchema(String),
    /// The local SQLite index cannot be used.
    Index(String),
    /// Synchronization could not complete; local durable work may still exist.
    Synchronization(String),
    /// A transport request is malformed.
    Protocol(String),
    /// An unexpected failure that adapters may map separately.
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotInitialized => formatter.write_str("project is not initialized"),
            Error::InvalidIdentity(detail)
            | Error::Persistence(detail)
            | Error::Validation(detail)
            | Error::RevisionConflict(detail)
            | Error::NotFound(detail)
            | Error::IncompatibleSchema(detail)
            | Error::Index(detail)
            | Error::Synchronization(detail)
            | Error::Protocol(detail)
            | Error::Internal(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn malformed(reason: &str) -> Error {
    Error::InvalidIdentity(format!("identity manifest is malformed: {reason}"))
}

pub(crate) fn unsupported_version(version: u64) -> Error {
    Error::InvalidIdentity(format!(
        "identity manifest version {version} is not supported; leave the file unchanged and upgrade the client"
    ))
}

pub(crate) fn persistence(context: &str, error: std::io::Error) -> Error {
    Error::Persistence(format!("{context}: {error}"))
}

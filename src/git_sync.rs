//! Git synchronization for authoritative Markdown files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Error;

/// Git operations over one local clone of the memory repository.
#[derive(Debug)]
pub struct GitSync {
    repository: PathBuf,
}

impl GitSync {
    /// Clone a memory repository when the machine has no local copy yet.
    pub fn clone_if_missing(remote: &str, destination: &Path) -> Result<Self, Error> {
        if destination.join(".git").is_dir() {
            return Ok(Self {
                repository: destination.to_path_buf(),
            });
        }
        if destination.exists()
            && fs::read_dir(destination)
                .map_err(persistence)?
                .next()
                .is_some()
        {
            return Err(Error::Synchronization(format!(
                "cannot clone into non-empty {}",
                destination.display()
            )));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(persistence)?;
        }
        run_git(None, &["clone", remote, &destination.to_string_lossy()])?;
        restrict_directory(destination)?;
        Ok(Self {
            repository: destination.to_path_buf(),
        })
    }

    /// Bind to an already cloned memory repository.
    pub fn open(repository: &Path) -> Result<Self, Error> {
        if !repository.join(".git").is_dir() {
            return Err(Error::Synchronization(format!(
                "memory repository is not a Git clone: {}",
                repository.display()
            )));
        }
        Ok(Self {
            repository: repository.to_path_buf(),
        })
    }

    /// Pull remote history without rewriting local history.
    pub fn pull(&self) -> Result<(), Error> {
        run_git(Some(&self.repository), &["pull", "--ff-only"])
    }

    /// Commit exactly the authoritative paths supplied by the application core.
    pub fn commit(&self, paths: &[&Path], message: &str) -> Result<(), Error> {
        if !message.contains('(') || !message.contains("): ") {
            return Err(Error::Validation(
                "Git commit message must use type(scope): English summary".to_string(),
            ));
        }
        let mut add = vec!["add", "--"];
        let rendered = paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        add.extend(rendered.iter().map(String::as_str));
        run_git(Some(&self.repository), &add)?;
        run_git(Some(&self.repository), &["commit", "-m", message])
    }

    /// Push local commits, retaining them if the remote is unavailable.
    pub fn push(&self) -> Result<(), Error> {
        run_git(Some(&self.repository), &["push"])
    }

    /// Synchronize remote history before subsequent reads or writes.
    pub fn synchronize(&self) -> Result<(), Error> {
        self.pull()?;
        self.push()
    }

    /// Repository directory used for this operation.
    #[must_use]
    pub fn repository(&self) -> &Path {
        &self.repository
    }
}

fn run_git(directory: Option<&Path>, arguments: &[&str]) -> Result<(), Error> {
    let mut command = Command::new("git");
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let output = command.output().map_err(persistence)?;
    if output.status.success() {
        return Ok(());
    }
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    if diagnostic.contains("non-fast-forward")
        || diagnostic.contains("fetch first")
        || diagnostic.contains("CONFLICT")
    {
        return Err(Error::RevisionConflict("Git rejected synchronization because the remote history conflicts; resolve the affected Markdown files explicitly".to_string()));
    }
    Err(Error::Synchronization(
        "Git synchronization failed; the local history was left unchanged".to_string(),
    ))
}

fn persistence(error: std::io::Error) -> Error {
    Error::Persistence(format!("run Git: {error}"))
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(persistence)
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> Result<(), Error> {
    Ok(())
}

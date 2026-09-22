//! Project identity.
//!
//! The code repository holds one manifest, [`.agent-knowledge.toml`](MANIFEST_FILE_NAME).
//! The memory repository holds one directory per project id. Callers pass the
//! memory repository path; its name and location are not decided here.
//!
//! A manifest is written by creating a temporary sibling and renaming it into
//! place, so a reader never observes a half-written file.

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, malformed, persistence, unsupported_version};

/// File name of the identity manifest at the root of a code repository.
pub const MANIFEST_FILE_NAME: &str = ".agent-knowledge.toml";

/// Identity format version this client writes and accepts.
pub const FORMAT_VERSION: u64 = 1;

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const PARTIAL_SUFFIX: &str = ".partial";

/// Immutable id of one code repository's memory.
///
/// Produced by [`initialize`] or read back through [`Project::id`]. No normal
/// operation takes a project id as an argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectId(String);

impl ProjectId {
    /// The canonical lowercase UUID text stored in the manifest and used as
    /// the memory directory name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn generate() -> Result<Self, Error> {
        let mut bytes = [0_u8; 16];
        let mut source =
            File::open("/dev/urandom").map_err(|error| persistence("read /dev/urandom", error))?;
        std::io::Read::read_exact(&mut source, &mut bytes)
            .map_err(|error| persistence("read /dev/urandom", error))?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Self(format_uuid(&bytes)))
    }

    fn parse(value: &str) -> Result<Self, Error> {
        if !is_uuid_v4(value) {
            return Err(malformed(
                "id must be a lowercase UUID version 4, for example 550e8400-e29b-41d4-a716-446655440000",
            ));
        }
        Ok(Self(value.to_string()))
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A code repository whose identity manifest was resolved.
///
/// The handle is bound to one project. Record operations do not accept a
/// different project id.
#[derive(Debug)]
pub struct Project {
    id: ProjectId,
    created: String,
    updated: String,
    memory_directory: PathBuf,
}

impl Project {
    /// The id stored in the manifest.
    #[must_use]
    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    /// Creation timestamp from the manifest, `YYYY-MM-DDTHH:MM:SSZ`.
    #[must_use]
    pub fn created(&self) -> &str {
        &self.created
    }

    /// Update timestamp from the manifest, `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// Phase 1 does not rewrite a valid manifest, so this stays equal to
    /// [`Self::created`] until a later phase has a reason to touch the file.
    #[must_use]
    pub fn updated(&self) -> &str {
        &self.updated
    }

    /// Directory of this project's records inside the memory repository.
    ///
    /// The path is the one resolved for this handle. It is not a way to name
    /// a different project.
    #[must_use]
    pub fn memory_directory(&self) -> &Path {
        &self.memory_directory
    }

    /// Names of the record files stored for this project, sorted by name.
    ///
    /// Phase 1 has no text index. The memory directory is the isolation
    /// boundary: this method reads that directory and takes no project id.
    /// Symbolic links are not followed and are not returned.
    pub fn search(&self) -> Result<Vec<String>, Error> {
        let mut names = Vec::new();
        let entries = fs::read_dir(&self.memory_directory).map_err(|error| {
            persistence(&format!("read {}", self.memory_directory.display()), error)
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                persistence(&format!("read {}", self.memory_directory.display()), error)
            })?;
            let file_type = entry.file_type().map_err(|error| {
                persistence(&format!("inspect {}", entry.path().display()), error)
            })?;
            if !file_type.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                return Err(Error::Persistence(format!(
                    "record file name is not valid UTF-8 in {}",
                    self.memory_directory.display()
                )));
            };
            names.push(name.to_string());
        }
        names.sort_unstable();
        Ok(names)
    }
}

/// Create the identity manifest if the code repository does not have one.
///
/// A valid existing manifest is left unchanged and its id is returned. A
/// malformed manifest, or one whose `version` this client does not support,
/// is left byte-for-byte unchanged and returned as [`Error::InvalidIdentity`].
pub fn initialize(code_repository: &Path, memory_repository: &Path) -> Result<ProjectId, Error> {
    require_directory(code_repository, "code repository")?;
    if manifest_path(code_repository).exists() {
        let manifest = read_manifest(code_repository)?;
        ensure_project_directory(memory_repository, &manifest.id)?;
        return Ok(manifest.id);
    }

    let id = unused_id(memory_repository)?;
    let timestamp = utc_now()?;
    ensure_project_directory(memory_repository, &id)?;
    if let Err(error) = write_manifest_atomic(code_repository, &id, &timestamp, &timestamp) {
        remove_if_empty(&memory_repository.join(id.as_str()));
        return Err(error);
    }
    Ok(id)
}

/// Resolve the project bound to `code_repository`.
///
/// This is the client start. Without a manifest the error is
/// [`Error::NotInitialized`] and there is no [`Project`] on which to read or
/// write knowledge. An existing but unusable manifest is
/// [`Error::InvalidIdentity`] and is not modified.
pub fn start(code_repository: &Path, memory_repository: &Path) -> Result<Project, Error> {
    require_directory(code_repository, "code repository")?;
    if !manifest_path(code_repository).exists() {
        return Err(Error::NotInitialized);
    }
    let manifest = read_manifest(code_repository)?;
    let memory_directory = memory_repository.join(manifest.id.as_str());
    if !memory_directory.is_dir() {
        return Err(Error::Persistence(format!(
            "project memory directory is missing: {}",
            memory_directory.display()
        )));
    }
    Ok(Project {
        id: manifest.id,
        created: manifest.created,
        updated: manifest.updated,
        memory_directory,
    })
}

#[derive(Debug)]
struct Manifest {
    id: ProjectId,
    created: String,
    updated: String,
}

fn manifest_path(code_repository: &Path) -> PathBuf {
    code_repository.join(MANIFEST_FILE_NAME)
}

fn partial_path(code_repository: &Path) -> PathBuf {
    code_repository.join(format!("{MANIFEST_FILE_NAME}{PARTIAL_SUFFIX}"))
}

fn require_directory(path: &Path, what: &str) -> Result<(), Error> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(Error::Persistence(format!(
            "{what} does not exist: {}",
            path.display()
        )))
    }
}

fn unused_id(memory_repository: &Path) -> Result<ProjectId, Error> {
    for _ in 0..8 {
        let id = ProjectId::generate()?;
        let directory = memory_repository.join(id.as_str());
        if !directory.exists() {
            return Ok(id);
        }
    }
    Err(Error::Persistence(
        "could not allocate a project id whose memory directory is free".to_string(),
    ))
}

fn ensure_project_directory(memory_repository: &Path, id: &ProjectId) -> Result<(), Error> {
    if memory_repository.exists() {
        if !memory_repository.is_dir() {
            return Err(Error::Persistence(format!(
                "memory repository is not a directory: {}",
                memory_repository.display()
            )));
        }
    } else {
        create_private_dir(memory_repository)?;
    }
    let project_directory = memory_repository.join(id.as_str());
    if project_directory.exists() {
        if project_directory.is_dir() {
            return Ok(());
        }
        return Err(Error::Persistence(format!(
            "project memory path is not a directory: {}",
            project_directory.display()
        )));
    }
    create_private_dir(&project_directory)
}

fn create_private_dir(path: &Path) -> Result<(), Error> {
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|error| persistence(&format!("create {}", path.display()), error))
}

fn remove_if_empty(path: &Path) {
    let Ok(mut entries) = fs::read_dir(path) else {
        return;
    };
    if entries.next().is_none() {
        let _ = fs::remove_dir(path);
    }
}

fn write_manifest_atomic(
    code_repository: &Path,
    id: &ProjectId,
    created: &str,
    updated: &str,
) -> Result<(), Error> {
    let temporary = partial_path(code_repository);
    let write_result = write_temporary(&temporary, &render(id, created, updated));
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, manifest_path(code_repository)).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        persistence(
            &format!("rename manifest into {}", code_repository.display()),
            error,
        )
    })?;
    if let Ok(directory) = File::open(code_repository) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn write_temporary(path: &Path, contents: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|error| persistence(&format!("create {}", path.display()), error))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| persistence(&format!("write {}", path.display()), error))?;
    file.sync_all()
        .map_err(|error| persistence(&format!("sync {}", path.display()), error))?;
    Ok(())
}

fn render(id: &ProjectId, created: &str, updated: &str) -> String {
    format!(
        "id = \"{id}\"\nversion = {FORMAT_VERSION}\ncreated = \"{created}\"\nupdated = \"{updated}\"\n"
    )
}

fn read_manifest(code_repository: &Path) -> Result<Manifest, Error> {
    let path = manifest_path(code_repository);
    let metadata = fs::metadata(&path)
        .map_err(|error| persistence(&format!("read {}", path.display()), error))?;
    if !metadata.is_file() {
        return Err(malformed("manifest path is not a file"));
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(malformed("file is too large"));
    }
    let bytes =
        fs::read(&path).map_err(|error| persistence(&format!("read {}", path.display()), error))?;
    let text = String::from_utf8(bytes).map_err(|_| malformed("file is not UTF-8"))?;
    parse_manifest(&text)
}

fn parse_manifest(text: &str) -> Result<Manifest, Error> {
    let mut id = None;
    let mut version = None;
    let mut created = None;
    let mut updated = None;

    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(malformed(&format!(
                "line {line_number}: expected key = value"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "id" => set_once(&mut id, parse_id(value)?, line_number, key)?,
            "version" => set_once(&mut version, parse_version(value)?, line_number, key)?,
            "created" => set_once(&mut created, parse_timestamp(value)?, line_number, key)?,
            "updated" => set_once(&mut updated, parse_timestamp(value)?, line_number, key)?,
            _ => return Err(malformed(&format!("line {line_number}: unknown key {key}"))),
        }
    }

    let version = version.ok_or_else(|| malformed("missing version"))?;
    if version != FORMAT_VERSION {
        return Err(unsupported_version(version));
    }
    Ok(Manifest {
        id: id.ok_or_else(|| malformed("missing id"))?,
        created: created.ok_or_else(|| malformed("missing created"))?,
        updated: updated.ok_or_else(|| malformed("missing updated"))?,
    })
}

fn set_once<T>(slot: &mut Option<T>, value: T, line_number: usize, key: &str) -> Result<(), Error> {
    if slot.is_some() {
        return Err(malformed(&format!(
            "line {line_number}: duplicate key {key}"
        )));
    }
    *slot = Some(value);
    Ok(())
}

fn parse_id(value: &str) -> Result<ProjectId, Error> {
    ProjectId::parse(unquote(value)?)
}

fn parse_version(value: &str) -> Result<u64, Error> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(malformed("version must be an integer"));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(malformed("version must not have a leading zero"));
    }
    value
        .parse()
        .map_err(|_| malformed("version is out of range"))
}

fn parse_timestamp(value: &str) -> Result<String, Error> {
    let text = unquote(value)?;
    if !is_utc_timestamp(text) {
        return Err(malformed("timestamp must be YYYY-MM-DDTHH:MM:SSZ in UTC"));
    }
    Ok(text.to_string())
}

fn unquote(value: &str) -> Result<&str, Error> {
    let bytes = value.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
        return Err(malformed("expected a double-quoted string"));
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains(['\\', '"']) {
        return Err(malformed("escapes are not accepted in manifest strings"));
    }
    Ok(inner)
}

fn utc_now() -> Result<String, Error> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Persistence("system clock is before the Unix epoch".to_string()))?;
    Ok(format_utc(elapsed.as_secs()))
}

fn format_utc(seconds: u64) -> String {
    let days = seconds / 86_400;
    let time = seconds % 86_400;
    let hour = time / 3_600;
    let minute = (time % 3_600) / 60;
    let second = time % 60;
    let (year, month, day) = civil_from_unix_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`, with `days` counted from 1970-01-01.
fn civil_from_unix_days(days: u64) -> (i32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month as u32, day as u32)
}

fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    let Some(year) = parse_digits(&bytes[0..4]) else {
        return false;
    };
    let Some(month) = parse_digits(&bytes[5..7]) else {
        return false;
    };
    let Some(day) = parse_digits(&bytes[8..10]) else {
        return false;
    };
    let Some(hour) = parse_digits(&bytes[11..13]) else {
        return false;
    };
    let Some(minute) = parse_digits(&bytes[14..16]) else {
        return false;
    };
    let Some(second) = parse_digits(&bytes[17..19]) else {
        return false;
    };
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    (1..=days_in_month(year, month)).contains(&day)
}

fn parse_digits(bytes: &[u8]) -> Option<u32> {
    let mut value = 0_u32;
    for byte in bytes {
        let digit = byte.checked_sub(b'0')?;
        if digit > 9 {
            return None;
        }
        value = value * 10 + u32::from(digit);
    }
    Some(value)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn is_uuid_v4(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    let groups = [8, 4, 4, 4, 12];
    let mut index = 0;
    for (group_index, length) in groups.iter().enumerate() {
        if group_index > 0 {
            if bytes[index] != b'-' {
                return false;
            }
            index += 1;
        }
        for _ in 0..*length {
            if !bytes[index].is_ascii_hexdigit() || bytes[index].is_ascii_uppercase() {
                return false;
            }
            index += 1;
        }
    }
    bytes[14] == b'4' && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

fn format_uuid(bytes: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::{FORMAT_VERSION, format_utc, is_utc_timestamp, parse_manifest};

    #[test]
    fn unix_epoch_formats_as_utc() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert!(is_utc_timestamp("1970-01-01T00:00:00Z"));
        assert!(is_utc_timestamp("2024-02-29T23:59:59Z"));
        assert!(!is_utc_timestamp("2023-02-29T00:00:00Z"));
        assert!(!is_utc_timestamp("2026-09-22T18:00:00+00:00"));
    }

    #[test]
    fn unsupported_version_is_rejected_without_rewriting_fields() {
        let text = "\
id = \"550e8400-e29b-41d4-a716-446655440000\"
version = 2
created = \"2026-09-22T18:00:00Z\"
updated = \"2026-09-22T18:00:00Z\"
";
        let error = parse_manifest(text).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("version 2"));
        assert!(message.contains("not supported"));
        assert_ne!(FORMAT_VERSION, 2);
    }
}

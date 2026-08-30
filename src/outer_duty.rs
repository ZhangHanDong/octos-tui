//! `octoscode outer-duty` (OUTER_LOOP_REVIEW #38): the kernel lock behind
//! multi-outer primary-reviewer authority — a per-project, session-lifetime
//! OS-exclusive lock (fs2 `try_lock_exclusive`), blueprint-verified against
//! the octos `ServeDataDirLock` precedent (structural contention detection
//! via `fs2::lock_contended_error`, never string matching) and the #43 CLI
//! liveness probe.
//!
//! Scope: single-machine flock/LockFileEx filesystems only — NFS is out of
//! scope. No agent self-help takeover: a live lock is HELD; taking over means
//! the operator terminates the old holder, then a fresh acquire.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr, eyre};

/// One of exactly three machine-readable states `check` may print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DutyState {
    /// No holder (the lock file may not even exist).
    Vacant,
    /// Another live process holds the exclusive lock.
    Held,
    /// I/O or structural error — NEVER masqueraded as VACANT.
    Error,
}

impl DutyState {
    pub fn as_str(self) -> &'static str {
        match self {
            DutyState::Vacant => "VACANT",
            DutyState::Held => "HELD",
            DutyState::Error => "ERROR",
        }
    }
}

/// Canonicalize `--project` into the lock path:
/// `~/.octos/outer/duty/<hex(canonical path)>.lock`.
pub fn lock_path(project: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(project)
        .wrap_err_with(|| format!("cannot canonicalize project path: {}", project.display()))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher as _};
    canonical.hash(&mut hasher);
    let digest = format!("{:016x}", hasher.finish());
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Ok(Path::new(&home)
        .join(".octos")
        .join("outer")
        .join("duty")
        .join(format!("{digest}.lock")))
}

/// Hold guard: the fd IS the lock (lifetime-bound; released on drop / process
/// exit incl. SIGKILL).
pub struct DutyHold {
    _file: std::fs::File,
    pub lock_path: PathBuf,
}

/// Acquire the duty lock NONBLOCKING, or fail structurally on contention.
pub fn acquire(project: &Path) -> Result<DutyHold> {
    let path = lock_path(project)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create duty lock dir: {}", parent.display()))?;
    }
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
        .wrap_err_with(|| format!("failed to open duty lockfile: {}", path.display()))?;
    match fs2::FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(DutyHold {
            _file: file,
            lock_path: path,
        }),
        Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
            Err(eyre!(
                "outer-duty: HELD by another live holder for this project ({}) — \
                 no agent self-takeover: the operator terminates the old holder, \
                 then a fresh acquire",
                path.display()
            ))
        }
        Err(error) => Err(eyre::Report::new(error)
            .wrap_err(format!("failed to acquire duty lock: {}", path.display()))),
    }
}

/// Probe the lock WITHOUT disturbing a live holder: open the same file and
/// `try_lock_exclusive` — on a held lock this errors with the platform's
/// contended errno (the fd is dropped immediately on both arms, so a VACANT
/// probe releases instantly; a HELD probe never blocks, never steals).
pub fn check(project: &Path) -> DutyState {
    let path = match lock_path(project) {
        Ok(path) => path,
        Err(_) => return DutyState::Error,
    };
    if !path.exists() {
        // No file at all: genuinely vacant (a stale sidecar never changes
        // this — ownership lives in the fd, not the filesystem presence).
        return DutyState::Vacant;
    }
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return DutyState::Error;
        }
    }
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
    {
        Ok(file) => file,
        Err(_) => return DutyState::Error,
    };
    match fs2::FileExt::try_lock_exclusive(&file) {
        Ok(()) => {
            // Dropping the file releases the probe's lock immediately.
            DutyState::Vacant
        }
        Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
            DutyState::Held
        }
        Err(_) => DutyState::Error,
    }
}

/// Write the diagnostic metadata sidecar atomically: 0600 tempfile + fsync +
/// rename. DIAGNOSTIC ONLY — corruption never affects lock adjudication
/// (callers surface METADATA_CORRUPT but the lock state stands).
pub fn write_metadata(lock: &Path, signature: &str, duties: &str) -> Result<()> {
    let sidecar = lock.with_extension("meta");
    let payload = format!(
        "{{\"signature\":{:?},\"duties\":{:?},\"written_at_unix\":{}}}\n",
        signature,
        duties,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    if let Some(parent) = sidecar.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = sidecar.with_extension("meta.tmp");
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(payload.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, &sidecar)
        .wrap_err_with(|| format!("failed to rename metadata sidecar: {}", sidecar.display()))
}

/// Parse the sidecar for diagnostics; `None` means unreadable/corrupt
/// (the caller reports METADATA_CORRUPT — adjudication is unaffected).
pub fn read_metadata(lock: &Path) -> Option<serde_json::Value> {
    let sidecar = lock.with_extension("meta");
    let text = std::fs::read_to_string(sidecar).ok()?;
    serde_json::from_str(&text).ok()
}

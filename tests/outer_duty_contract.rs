//! Contract tests for `octoscode outer-duty` (OUTER_LOOP_REVIEW #38) — the
//! per-project session-lifetime OS-exclusive duty lock. Real subprocess
//! (cargo's CARGO_BIN_EXE), a temp project dir + a temp HOME (lock files
//! land in $HOME/.octos/outer/duty/ so the suite never touches real state).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_octoscode"))
}

struct TempHome(PathBuf);
impl TempHome {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "outer-duty-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(dir.join("home")).unwrap();
        std::fs::create_dir_all(dir.join("project")).unwrap();
        Self(dir)
    }
    fn home(&self) -> PathBuf {
        self.0.join("home")
    }
    fn project(&self) -> PathBuf {
        self.0.join("project")
    }
    fn lock_path(&self) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher as _};
        let canonical = std::fs::canonicalize(self.project()).unwrap();
        canonical.hash(&mut hasher);
        self.home()
            .join(".octos")
            .join("outer")
            .join("duty")
            .join(format!("{:016x}.lock", hasher.finish()))
    }
}
impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn duty(home: &std::path::Path) -> Command {
    let mut cmd = Command::new(bin());
    cmd.arg("outer-duty").env("HOME", home);
    cmd
}

/// Spawn `hold -- sleep 15` and wait until `check` reports HELD.
#[allow(clippy::zombie_processes)]
fn spawn_holder(home: &std::path::Path, project: &std::path::Path) -> std::process::Child {
    let mut cmd = duty(home);
    cmd.args([
        "hold",
        "--project",
        project.to_str().unwrap(),
        "--signature",
        "test-holder",
        "--duties",
        "primary-review",
        "--",
        "sleep",
        "15",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    let child = cmd.spawn().expect("spawn holder");
    // Wait for the lock to be taken (poll check until HELD).
    for _ in 0..100 {
        let out = duty(home)
            .args(["check", "--project", project.to_str().unwrap()])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&out.stdout)
            .trim()
            .starts_with("HELD")
        {
            return child;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("holder never acquired the lock");
}

/// 契约 1: two concurrent contenders — exactly one wins.
#[test]
fn duty_two_contenders_exactly_one_wins() {
    let env = TempHome::new("two");
    let mut a = spawn_holder(env.home().as_path(), env.project().as_path());
    // The second contender must be refused (nonblocking) with a nonzero code.
    let out = duty(env.home().as_path())
        .args([
            "hold",
            "--project",
            env.project().to_str().unwrap(),
            "--signature",
            "contender-b",
            "--duties",
            "x",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "second contender must lose");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("HELD"), "contention is explicit: {stderr}");
    let _ = a.kill();
    let _ = a.wait();
}

/// 契约 2: kill -9 the holder → the lock is instantly VACANT (fd-release,
/// no stale-lock hazard).
#[test]
fn duty_sigkill_releases_instantly() {
    let env = TempHome::new("kill");
    let mut holder = spawn_holder(env.home().as_path(), env.project().as_path());
    holder.kill().expect("kill -9");
    let _ = holder.wait();
    let out = duty(env.home().as_path())
        .args(["check", "--project", env.project().to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "VACANT");
}

/// 契约 3: `check` never disturbs a live holder (probe releases instantly;
/// the holder keeps the lock and finishes cleanly).
#[test]
fn duty_check_does_not_disturb_holder() {
    let env = TempHome::new("probe");
    let mut holder = spawn_holder(env.home().as_path(), env.project().as_path());
    // Probe repeatedly while the holder runs.
    for _ in 0..10 {
        let out = duty(env.home().as_path())
            .args(["check", "--project", env.project().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .starts_with("HELD"),
            "probe sees held: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
    // Kill the holder AFTER the probes: it must still have held the lock the
    // whole time (probes never stole it) — its death now yields VACANT.
    holder.kill().unwrap();
    let _ = holder.wait();
    let out = duty(env.home().as_path())
        .args(["check", "--project", env.project().to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "VACANT");
}

/// 契约 4: a corrupt metadata sidecar never changes ownership — check still
/// adjudicates HELD (with METADATA_CORRUPT note) or VACANT per the fd.
#[test]
fn duty_corrupt_metadata_keeps_ownership() {
    let env = TempHome::new("meta");
    let mut holder = spawn_holder(env.home().as_path(), env.project().as_path());
    // Corrupt the sidecar while held.
    let lock = env.lock_path();
    std::fs::write(lock.with_extension("meta"), "{not json").unwrap();
    let out = duty(env.home().as_path())
        .args(["check", "--project", env.project().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim().starts_with("HELD"),
        "corrupt sidecar must not flip the verdict: {stdout}"
    );
    assert!(stdout.contains("METADATA_CORRUPT"), "{stdout}");
    holder.kill().unwrap();
    let _ = holder.wait();
    // After the holder dies: VACANT — still unaffected by the corrupt file.
    let out = duty(env.home().as_path())
        .args(["check", "--project", env.project().to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "VACANT");
}

/// 契约 5: the lock file and the sidecar are created with tight permissions
/// (0600) even under a permissive umask.
#[test]
fn duty_files_are_0600_under_permissive_umask() {
    let env = TempHome::new("perm");
    let mut holder = spawn_holder(env.home().as_path(), env.project().as_path());
    let lock = env.lock_path();
    let meta = lock.with_extension("meta");
    assert!(lock.exists(), "lockfile at {}", lock.display());
    for _ in 0..100 {
        if meta.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(meta.exists(), "sidecar at {}", meta.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let lock_mode = std::fs::metadata(&lock).unwrap().permissions().mode() & 0o777;
        let meta_mode = std::fs::metadata(&meta).unwrap().permissions().mode() & 0o777;
        assert_eq!(lock_mode, 0o600, "lockfile mode {lock_mode:o}");
        assert_eq!(meta_mode, 0o600, "sidecar mode {meta_mode:o}");
    }
    holder.kill().unwrap();
    let _ = holder.wait();
}

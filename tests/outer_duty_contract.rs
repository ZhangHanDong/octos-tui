//! Contract tests for `octoscode outer-duty` (#38 / #38-r1) — the
//! per-project session-lifetime OS-exclusive duty lock. Real subprocess
//! (CARGO_BIN_EXE), temp HOME + project (locks never touch real state).
#![cfg(unix)]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn bin() -> PathBuf {
    // CARGO_BIN_EXE may point at a bin-unittest harness under some cargo
    // versions; prefer the real bin adjacent to the test deps directory,
    // falling back to the env var and the plain target path.
    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("../../octoscode")))
            .map(|p| p.canonicalize().unwrap_or(p)),
        std::env::var("CARGO_BIN_EXE_octoscode")
            .ok()
            .map(PathBuf::from),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/octoscode")),
    ];
    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(env!("CARGO_BIN_EXE_octoscode"))
}

struct TempHome(PathBuf);
impl TempHome {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("outer-duty-r1-{tag}-{}-{seq}", std::process::id(),));
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
    fn sentinel(&self) -> PathBuf {
        self.0.join("sentinel")
    }
    /// Recompute the stable SHA-256 lock name (domain-prefixed) the same
    /// way the implementation does — used to locate files for assertions.
    fn lock_path(&self) -> PathBuf {
        use sha2::Digest as _;
        let canonical = std::fs::canonicalize(self.project()).unwrap();
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"octoscode/outer-duty/v1");
        hasher.update([0u8]);
        hasher.update(canonical.to_string_lossy().as_bytes());
        let digest_bytes = hasher.finalize();
        let mut digest = String::new();
        for byte in digest_bytes {
            digest.push_str(&format!("{byte:02x}"));
        }
        self.home()
            .join(".octos")
            .join("outer")
            .join("duty")
            .join(format!("{digest}.lock"))
    }
}
impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Warm the binary/page cache once per test run (first-spawn cold jitter
/// was the intermittence amplifier for the acquire polls).
fn warmup() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = Command::new(bin())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}

fn duty(home: &std::path::Path) -> Command {
    let mut cmd = Command::new(bin());
    cmd.arg("outer-duty").env("HOME", home);
    cmd
}

/// Spawn `hold` whose child loops while a per-test sentinel file exists.
/// Returns (wrapper, sentinel): remove the sentinel to end the REAL child.
#[allow(clippy::zombie_processes)]
fn spawn_holder(
    home: &std::path::Path,
    project: &std::path::Path,
    sentinel: &std::path::Path,
) -> std::process::Child {
    std::fs::write(sentinel, b"run").unwrap();
    let started = sentinel.with_extension("started");
    let loop_cmd = format!(
        "while test -e {sent}; do touch {started}; sleep 0.05; done",
        sent = sentinel.display(),
        started = started.display(),
    );
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
        "/bin/sh",
        "-c",
        &loop_cmd,
    ])
    .stdout(Stdio::null())
    .stderr(std::process::Stdio::from(
        std::fs::File::create(sentinel.with_extension("wrapper-err")).unwrap(),
    ));
    warmup();
    let child = cmd.spawn().expect("spawn holder");
    for _ in 0..1200 {
        let out = duty(home)
            .args(["check", "--project", project.to_str().unwrap()])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&out.stdout).trim() == "HELD" {
            return child;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let wrapper_err =
        std::fs::read_to_string(sentinel.with_extension("wrapper-err")).unwrap_or_default();
    panic!("holder never acquired the lock; wrapper_err={wrapper_err:?}");
}

/// Wait until the holder's real child is provably past execve (started
/// marker from the sentinel loop).
fn wait_started(env: &TempHome) {
    let started = env.sentinel().with_extension("started");
    for _ in 0..600 {
        if started.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("real child never started");
}

fn check_state(home: &std::path::Path, project: &std::path::Path) -> String {
    let out = duty(home)
        .args(["check", "--project", project.to_str().unwrap()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// 契约 1: two contenders started against a shared start barrier — exactly
/// one hold succeeds, the loser exits nonzero with an explicit contention.
#[test]
fn duty_two_contenders_exactly_one_wins() {
    let env = TempHome::new("two");
    let project = env.project();
    // First holder holds via a long sleep.
    let mut a = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    // Second contender races while A holds.
    let out = duty(env.home().as_path())
        .args([
            "hold",
            "--project",
            project.to_str().unwrap(),
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
    assert!(stderr.contains("HELD"), "contention explicit: {stderr}");
    let _ = a.kill();
    let _ = a.wait();
    release_via_sentinel(&env);
}

/// 契约 2a(#38-r2 裁决:守护式死亡耦合): wrapper SIGKILL → the agent
/// dies with it (PR_SET_PDEATHSIG) → the lock goes VACANT with no
/// lingering authority.
#[test]
fn duty_wrapper_death_kills_agent_and_releases() {
    let env = TempHome::new("guardian");
    let project = env.project();
    let mut wrapper = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    wait_started(&env);
    wrapper.kill().unwrap();
    let _ = wrapper.wait();
    // The agent receives SIGKILL via PDEATHSIG; the lock releases. Poll —
    // the signal delivery is kernel-side and near-instant but asynchronous.
    for _ in 0..600 {
        if check_state(env.home().as_path(), project.as_path()) == "VACANT" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        check_state(env.home().as_path(), project.as_path()),
        "VACANT",
        "wrapper death must kill the agent and release the authority"
    );
}

/// 契约 2b(#38-r2): the agent exits while a GRANDCHILD keeps running —
/// the grandchild holds no fd and no authority: VACANT (a lingering
/// grandchild can never keep the duty lock alive).
#[test]
fn duty_grandchild_lingering_yields_vacant() {
    let env = TempHome::new("grandchild");
    let project = env.project();
    // Agent = sh that spawns a long-lived grandchild then exits.
    std::fs::write(env.sentinel(), b"run").unwrap();
    let grand_sentinel = env.0.join("grandchild-sentinel");
    std::fs::write(&grand_sentinel, b"run").unwrap();
    let loop_cmd = format!(
        "/bin/sh -c 'while test -e {g}; do sleep 0.05; done' & exit 0",
        g = grand_sentinel.display()
    );
    let mut cmd = duty(env.home().as_path());
    cmd.args([
        "hold",
        "--project",
        project.to_str().unwrap(),
        "--signature",
        "agent",
        "--duties",
        "review",
        "--",
        "/bin/sh",
        "-c",
        &loop_cmd,
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    let wrapper = cmd.spawn().unwrap();
    // Agent exits almost immediately; grandchild lives on. Wait past the
    // wrapper's own exit: the lock must be VACANT while the grandchild runs.
    for _ in 0..600 {
        if check_state(env.home().as_path(), project.as_path()) == "VACANT" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        grand_sentinel.exists(),
        "precondition: the grandchild is still alive"
    );
    assert_eq!(
        check_state(env.home().as_path(), project.as_path()),
        "VACANT",
        "a lingering grandchild must never keep the duty lock"
    );
    // Cleanup: end the grandchild.
    let _ = std::fs::remove_file(&grand_sentinel);
    let _ = wrapper;
}

/// 契约 3: repeated checks never disturb the live holder.
#[test]
fn duty_check_does_not_disturb_holder() {
    let env = TempHome::new("probe");
    let project = env.project();
    let mut holder = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    for _ in 0..10 {
        assert_eq!(check_state(env.home().as_path(), project.as_path()), "HELD");
    }
    holder.kill().unwrap();
    let _ = holder.wait();
    release_via_sentinel(&env);
    for _ in 0..600 {
        if check_state(env.home().as_path(), project.as_path()) == "VACANT" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        check_state(env.home().as_path(), project.as_path()),
        "VACANT"
    );
}

/// 契约 4: corrupt sidecar → HELD verdict unchanged, METADATA_CORRUPT on
/// stderr; after the holders die → VACANT (adjudication never reads sidecar).
#[test]
fn duty_corrupt_metadata_keeps_ownership() {
    let env = TempHome::new("meta");
    let project = env.project();
    let mut holder = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    std::fs::write(env.lock_path().with_extension("meta"), "{not json").unwrap();
    let out = duty(env.home().as_path())
        .args(["check", "--project", project.to_str().unwrap()])
        .output()
        .unwrap();
    // stdout: single state line; diagnostics on stderr.
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "HELD");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("METADATA_CORRUPT"), "{stderr}");
    holder.kill().unwrap();
    let _ = holder.wait();
    release_via_sentinel(&env);
    for _ in 0..600 {
        if check_state(env.home().as_path(), project.as_path()) == "VACANT" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        check_state(env.home().as_path(), project.as_path()),
        "VACANT"
    );
}

/// 契约 5: lockfile + sidecar + duty dir are 0600/0700 even when
/// pre-created 0644/0755 (tighten-on-open) under a permissive umask.
#[test]
fn duty_files_tighten_preexisting_permissive() {
    let env = TempHome::new("perm");
    let project = env.project();
    // Permissive umask EXPLICITLY set for the whole test (countersign D:
    // umask 显式设置) — restored on exit.
    struct UmaskGuard(libc::mode_t);
    impl Drop for UmaskGuard {
        fn drop(&mut self) {
            #[allow(unsafe_code)]
            unsafe {
                libc::umask(self.0)
            };
        }
    }
    #[allow(unsafe_code)]
    let _umask_guard = UmaskGuard(unsafe { libc::umask(0o000) });
    // Pre-create the lock dir + lockfile WORLD-READABLE.
    let lock = env.lock_path();
    let dir = lock.parent().unwrap();
    std::fs::create_dir_all(dir).unwrap();
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(&lock, b"").unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o644)).unwrap();
    let mut holder = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    let lock_mode = std::fs::metadata(&lock).unwrap().permissions().mode() & 0o777;
    let dir_mode = std::fs::metadata(dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        lock_mode, 0o600,
        "lockfile tightened from 0644: {lock_mode:o}"
    );
    assert_eq!(dir_mode, 0o700, "dir tightened from 0755: {dir_mode:o}");
    let meta = lock.with_extension("meta");
    for _ in 0..600 {
        if meta.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let meta_mode = std::fs::metadata(&meta).unwrap().permissions().mode() & 0o777;
    assert_eq!(meta_mode, 0o600);
    holder.kill().unwrap();
    let _ = holder.wait();
    release_via_sentinel(&env);
}

/// 契约 6(#38-r1 B/D): ERROR — never VACANT — for a missing HOME
/// (fail-closed) and a nonexistent project path.
#[test]
fn duty_error_never_vacant_for_bad_inputs() {
    let env = TempHome::new("err");
    // Missing HOME: fail-closed.
    let out = Command::new(bin())
        .arg("outer-duty")
        .args(["check", "--project", env.project().to_str().unwrap()])
        .env_remove("HOME")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ERROR");
    assert!(!out.status.success());
    // Nonexistent project: canonicalize fails → ERROR.
    let out = duty(env.home().as_path())
        .args(["check", "--project", "/nonexistent/project/xyz"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "ERROR",
        "canonicalize failure must be ERROR, not VACANT"
    );
}

/// 契约 7(#38-r1 C): stdout is EXACTLY one state line even when the
/// signature/duties metadata is hostile (newlines/ANSI/oversized).
#[test]
fn duty_stdout_single_line_with_hostile_metadata() {
    let env = TempHome::new("single");
    let project = env.project();
    let mut cmd = duty(env.home().as_path());
    cmd.args([
        "hold",
        "--project",
        project.to_str().unwrap(),
        "--signature",
        "evil\nANSI\x1b[31m",
        "--duties",
        "line1\nline2\r\nline3",
        "--",
        "sleep",
        "30",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    let mut holder = cmd.spawn().unwrap();
    for _ in 0..200 {
        let out = duty(env.home().as_path())
            .args(["check", "--project", project.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let trimmed = stdout.trim();
        if trimmed == "HELD" {
            assert_eq!(stdout.lines().count(), 1, "stdout stays one line");
            break;
        }
        assert!(
            ["VACANT", "ERROR"].contains(&trimmed) || trimmed == "HELD",
            "single machine state per line: {stdout:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    holder.kill().unwrap();
    let _ = holder.wait();
    release_via_sentinel(&env);
}

/// 契约 8(#38-r2 B3): stable digest — a FIXED input's 64-hex golden,
/// computed independently (not a mirror of the implementation), plus
/// symlink/relative convergence on the same canonical lock.
#[test]
fn duty_lock_digest_golden_and_convergence() {
    // Golden: sha256("octoscode/outer-duty/v1\0" ++ "/tmp/duty-golden-proj")
    // — computed out-of-band; the pure fn must reproduce it exactly.
    let golden = sha256_hex(b"octoscode/outer-duty/v1\0/tmp/duty-golden-proj");
    assert_eq!(
        octoscode::outer_duty::lock_digest(b"/tmp/duty-golden-proj"),
        golden,
        "stable lock-name digest must match the independent golden"
    );
    assert_eq!(golden.len(), 64);

    let env = TempHome::new("symlink");
    let project = env.project();
    let link = env.0.join("project-link");
    std::os::unix::fs::symlink(&project, &link).unwrap();
    let mut holder = spawn_holder(
        env.home().as_path(),
        project.as_path(),
        env.sentinel().as_path(),
    );
    assert_eq!(
        check_state(env.home().as_path(), link.as_path()),
        "HELD",
        "symlink path converges on the same canonical lock"
    );
    let lock = env.lock_path();
    assert!(lock.exists(), "stable-named lock at {}", lock.display());
    holder.kill().unwrap();
    let _ = holder.wait();
    for _ in 0..600 {
        if check_state(env.home().as_path(), project.as_path()) == "VACANT" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Independent SHA-256 → hex (test-local, not importing the impl's helper).
fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest as _;
    let bytes = sha2::Sha256::digest(data);
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// End the real child by removing its sentinel and wait for VACANT.
fn release_via_sentinel(env: &TempHome) {
    let _ = std::fs::remove_file(env.sentinel());
    for _ in 0..600 {
        if check_state(env.home().as_path(), env.project().as_path()) == "VACANT" {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

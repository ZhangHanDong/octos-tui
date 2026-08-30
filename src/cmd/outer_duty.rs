//! `octoscode outer-duty` (OUTER_LOOP_REVIEW #38): hold/check entrypoints
//! over [`crate::outer_duty`].

use super::OuterDutyArgs;
use crate::outer_duty::{self, DutyState};

/// Run the subcommand; returns the process exit code.
pub fn run(args: OuterDutyArgs) -> i32 {
    let project = std::path::PathBuf::from(&args.project);
    match args.action.as_str() {
        "check" => run_check(&project),
        "hold" => run_hold(&args, &project),
        other => {
            eprintln!("outer-duty: unknown action {other:?} (expected hold|check)");
            2
        }
    }
}

fn run_check(project: &std::path::Path) -> i32 {
    match outer_duty::check(project) {
        DutyState::Vacant => {
            println!("VACANT");
            0
        }
        DutyState::Held => {
            // Diagnostics only — a corrupt sidecar reports METADATA_CORRUPT
            // but NEVER changes the lock verdict.
            let mut note = String::new();
            if let Ok(path) = outer_duty::lock_path(project) {
                match outer_duty::read_metadata(&path) {
                    Some(meta) => {
                        note = format!(
                            " holder={} duties={}",
                            meta.get("signature")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?"),
                            meta.get("duties").and_then(|v| v.as_str()).unwrap_or("?"),
                        )
                    }
                    None => note = " METADATA_CORRUPT".into(),
                }
            }
            println!("HELD{note}");
            0
        }
        DutyState::Error => {
            println!("ERROR");
            1
        }
    }
}

fn run_hold(args: &OuterDutyArgs, project: &std::path::Path) -> i32 {
    if args.command.is_empty() {
        eprintln!("outer-duty hold: a child command after `--` is required");
        return 2;
    }
    let hold = match outer_duty::acquire(project) {
        Ok(hold) => hold,
        Err(err) => {
            eprintln!("outer-duty hold: {err:#}");
            return 1;
        }
    };
    // Diagnostic sidecar (best-effort; corruption never affects the lock).
    let _ = outer_duty::write_metadata(&hold.lock_path, &args.signature, &args.duties);
    // The fd lives in `hold` for the child's entire lifetime: lock-with-the-
    // holder, auto-released on exit/SIGKILL.
    let status = match std::process::Command::new(&args.command[0])
        .args(&args.command[1..])
        .status()
    {
        Ok(status) => status,
        Err(err) => {
            eprintln!("outer-duty hold: failed to spawn child: {err}");
            return 1;
        }
    };
    code_of(status)
}

#[cfg(unix)]
fn code_of(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt as _;
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0))
}

#[cfg(not(unix))]
fn code_of(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

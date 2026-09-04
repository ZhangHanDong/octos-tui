//! Evolution loop phase-1 retro contract tests (#42, SDD spec:
//! `specs/task-req-olp-evo-p1.spec.md`). 15 test functions whose names
//! match the contract's scenario filters verbatim; skeleton compiles
//! with `#[ignore]` so `-- --list` shows all of them (42a).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[allow(dead_code)] // exercised from 42b onward
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[allow(dead_code)] // exercised from 42b onward
fn script() -> PathBuf {
    repo_root().join("scripts/olp-evo-retro.sh")
}

#[allow(dead_code)] // exercised from 42b onward
static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
    repo: PathBuf,
    state_root: PathBuf,
}

#[allow(dead_code)] // exercised from 42b onward
impl Sandbox {
    fn new(tag: &str) -> Self {
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("olp-evo-retro-{tag}-{}-{seq}", std::process::id()));
        let repo = root.join("repo");
        let state_root = root.join("state");
        std::fs::create_dir_all(repo.join(".octos")).unwrap();
        std::fs::create_dir_all(&state_root).unwrap();
        Self {
            root,
            repo,
            state_root,
        }
    }

    fn evo_board(&self) -> PathBuf {
        self.repo.join(".octos/EVOLUTION.md")
    }

    fn install_board(&self, fixture: &str) {
        let src = repo_root().join("fixtures/evolution/retro").join(fixture);
        std::fs::copy(src, self.evo_board()).unwrap();
    }

    fn run(&self, dry_run: bool) -> Output {
        let mut cmd = Command::new("bash");
        cmd.arg(script()).arg(&self.repo);
        if dry_run {
            cmd.arg("--dry-run");
        }
        cmd.env("OLP_EVO_STATE", &self.state_root);
        cmd.output().unwrap()
    }

    fn run_env(&self, extra: &[(&str, &str)]) -> Output {
        let mut cmd = Command::new("bash");
        cmd.arg(script())
            .arg(&self.repo)
            .env("OLP_EVO_STATE", &self.state_root);
        for (k, v) in extra {
            cmd.env(k, v);
        }
        cmd.output().unwrap()
    }

    fn project_dir(&self) -> PathBuf {
        let mut it = std::fs::read_dir(&self.state_root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir());
        it.next()
            .map(|e| e.path())
            .unwrap_or_else(|| self.state_root.clone())
    }

    fn retro_json(&self) -> PathBuf {
        self.project_dir().join("retro.json")
    }

    fn brief_text(&self) -> String {
        let brief = latest_brief(&self.retro_json());
        std::fs::read_to_string(brief).unwrap_or_default()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[allow(dead_code)] // exercised from 42b onward
fn latest_brief(retro_json: &Path) -> PathBuf {
    let text = std::fs::read_to_string(retro_json).unwrap_or_default();
    let mut best = PathBuf::new();
    // last run entry's brief path
    for line in text.lines() {
        if let Some(pos) = line.find("\"brief\":\"") {
            let rest = &line[pos + 9..];
            if let Some(end) = rest.find('"') {
                best = PathBuf::from(&rest[..end]);
            }
        }
    }
    best
}

/// Scenario: 不同错误码不合并
#[test]
#[ignore]
fn olp_evo_retro_error_codes_are_distinct_candidates() {}

/// Scenario: events 卡按 detail 分组
#[test]
#[ignore]
fn olp_evo_retro_events_group_by_detail() {}

/// Scenario: 仅数字与路径不同的卡合并并数出锚点
#[test]
#[ignore]
fn olp_evo_retro_merges_num_path_variants_and_counts_anchors() {}

/// Scenario: 路径含井号时锚点仍正确
#[test]
#[ignore]
fn olp_evo_retro_anchor_rsplit_survives_hash_in_path() {}

/// Scenario: 锚点为减号的卡各计一次
#[test]
#[ignore]
fn olp_evo_retro_dash_anchor_counts_each_card() {}

/// Scenario: 草稿标注 draft 且用下一个 FLAW 编号
#[test]
#[ignore]
fn olp_evo_retro_draft_marks_todo_and_next_flaw_id() {}

/// Scenario: 简报与状态 schema 完整
#[test]
#[ignore]
fn olp_evo_retro_brief_and_runs_schema() {}

/// Scenario: 游标推进后重跑零新卡
#[test]
#[ignore]
fn olp_evo_retro_cursor_advances_and_rerun_is_empty() {}

/// Scenario: 并发两次运行不丢记录
#[test]
#[ignore]
fn olp_evo_retro_concurrent_runs_keep_records() {}

/// Scenario: 简报落盘后写游标前崩溃可恢复
#[test]
#[ignore]
fn olp_evo_retro_recovers_after_crash_before_cursor() {}

/// Scenario: dry-run 零写入
#[test]
#[ignore]
fn olp_evo_retro_dry_run_writes_nothing() {}

/// Scenario: 畸形卡被报告一次并跳过
#[test]
#[ignore]
fn olp_evo_retro_malformed_card_reported_once() {}

/// Scenario: 无新卡退出 0
#[test]
#[ignore]
fn olp_evo_retro_no_cards_exit_zero() {}

/// Scenario: skill 卡 outer 第 5 步就位且受保护章节原文不变
#[test]
#[ignore]
fn olp_evo_retro_skill_step5_and_protected_sections_golden() {}

/// Scenario: BOOT §7 新增定式且 §0 至 §6 原文不变
#[test]
#[ignore]
fn olp_evo_retro_boot_section7_and_golden() {}

//! Contract tests for specs/task-startup-splash.spec.

use octoscode::splash::{SPLASH_EFFECTS, SplashGate, pick_effect_name, should_play, splash_text};

/// A gate whose every condition allows playback; tests flip one field each.
fn open_gate() -> SplashGate {
    SplashGate {
        no_splash_flag: false,
        env_disabled: false,
        stdout_is_tty: true,
        ci: false,
        term_cols: 120,
        term_rows: 40,
    }
}

#[test]
fn should_play_true_when_interactive_and_wide_enough() {
    assert!(should_play(&open_gate()));
}

#[test]
fn should_play_false_when_stdout_not_tty() {
    let gate = SplashGate {
        stdout_is_tty: false,
        ..open_gate()
    };
    assert!(!should_play(&gate));
}

#[test]
fn should_play_false_on_flag_or_env() {
    let gate = SplashGate {
        no_splash_flag: true,
        ..open_gate()
    };
    assert!(!should_play(&gate));
    let gate = SplashGate {
        env_disabled: true,
        ..open_gate()
    };
    assert!(!should_play(&gate));
}

#[test]
fn should_play_false_in_ci() {
    let gate = SplashGate {
        ci: true,
        ..open_gate()
    };
    assert!(!should_play(&gate));
}

#[test]
fn should_play_false_when_terminal_narrower_than_logo() {
    let gate = SplashGate {
        term_cols: 30,
        ..open_gate()
    };
    assert!(!should_play(&gate));
    let gate = SplashGate {
        term_rows: 5,
        ..open_gate()
    };
    assert!(!should_play(&gate));
}

#[test]
fn pick_effect_stays_in_curated_list() {
    for seed in 0..64u64 {
        let name = pick_effect_name(seed);
        assert!(
            SPLASH_EFFECTS.contains(&name),
            "seed {seed} picked {name}, not in SPLASH_EFFECTS"
        );
    }
}

#[test]
fn splash_text_carries_logo_and_version() {
    let text = splash_text();
    assert!(text.contains(env!("CARGO_PKG_VERSION")));
    assert!(text.lines().count() >= 6, "logo should be multi-line");
}

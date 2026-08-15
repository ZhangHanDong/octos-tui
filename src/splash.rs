//! Startup splash: a ttfx-rendered OCTOS logo animation played on the main
//! screen before the event loop claims the terminal.
//! Contract: specs/task-startup-splash.spec.

use unicode_width::UnicodeWidthStr;

/// Block-letter OCTOS. 44 columns wide, 6 rows tall; all glyphs are
/// single-width so ttfx canvas geometry matches `lines()`/width math.
const LOGO: &str = "\
  ██████╗  ██████╗████████╗ ██████╗ ███████╗
 ██╔═══██╗██╔════╝╚══██╔══╝██╔═══██╗██╔════╝
 ██║   ██║██║        ██║   ██║   ██║███████╗
 ██║   ██║██║        ██║   ██║   ██║╚════██║
 ╚██████╔╝╚██████╗   ██║   ╚██████╔╝███████║
  ╚═════╝  ╚═════╝   ╚═╝    ╚═════╝ ╚══════╝";

/// Curated effects, chosen for reading well when truncated at ~1.5s.
/// Members may be tuned after visual testing; keep them valid ttfx
/// subcommand names (`ttfx <name>` must parse).
pub const SPLASH_EFFECTS: [&str; 6] = ["decrypt", "beams", "sweep", "wipe", "slice", "expand"];

/// The animated input: logo plus a version footer line.
pub fn splash_text() -> String {
    format!(
        "{LOGO}\n\n         octoscode v{}",
        env!("CARGO_PKG_VERSION")
    )
}

/// Widest line / line count of the splash text, for the gate and printer.
fn text_dimensions(text: &str) -> (u16, u16) {
    let cols = text.lines().map(UnicodeWidthStr::width).max().unwrap_or(0);
    let rows = text.lines().count();
    (cols as u16, rows as u16)
}

/// Every input to the play/skip decision, resolved by the caller so the
/// decision itself is a pure function (spec: 门控).
#[derive(Debug, Clone, Copy)]
pub struct SplashGate {
    pub no_splash_flag: bool,
    /// OCTOSCODE_NO_SPLASH is set (any value).
    pub env_disabled: bool,
    pub stdout_is_tty: bool,
    /// CI env var is set (any value).
    pub ci: bool,
    pub term_cols: u16,
    pub term_rows: u16,
}

/// All skip conditions are an unordered OR; any hit disables the splash.
pub fn should_play(gate: &SplashGate) -> bool {
    let (logo_cols, logo_rows) = text_dimensions(&splash_text());
    !gate.no_splash_flag
        && !gate.env_disabled
        && gate.stdout_is_tty
        && !gate.ci
        && gate.term_cols >= logo_cols
        // +2: one row below the canvas for the parked cursor, one of headroom.
        && gate.term_rows >= logo_rows + 2
}

/// Deterministic pick from SPLASH_EFFECTS (seeded ttfx Rng, unit-testable).
pub fn pick_effect_name(seed: u64) -> &'static str {
    let mut rng = ttfx::utils::rng::Rng::seeded(seed);
    SPLASH_EFFECTS[rng.choice_index(SPLASH_EFFECTS.len())]
}

//! Terminal mode-detection probes — TTY kind, alternate-screen support, and
//! colour depth — as small, parser-free helpers that return typed enums.
//!
//! These are **core** API (not behind the `lifecycle` feature flag) per
//! D-TUIN-002: every consumer that decides how to render needs them, and they
//! carry zero new dependencies (stdlib [`std::io::IsTerminal`] + the
//! [`std::env`](mod@std::env) module).
//! Returning typed enums rather than raw capability bits forces consumers to
//! handle the cases the crate decides matter (the [`ColourDepth`] steps, the
//! [`TtyKind`] variants) instead of leaking probe internals.
//!
//! The detection logic is split into pure `resolve` functions that take their
//! inputs explicitly — so the full matrix is unit-testable without touching the
//! real process environment — and thin `detect`/`of` wrappers that read the
//! live environment and standard streams.

use std::io::IsTerminal;

/// Whether a stream is connected to an interactive terminal.
///
/// # Stability
///
/// **unstable** (D-TUIN-005). Recently added; the surface may change before it
/// is graded stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TtyKind {
    /// The stream is an interactive terminal (a human is likely watching).
    Interactive,
    /// The stream is redirected — a pipe, file, or non-TTY (CI, `| cat`, …).
    NonInteractive,
}

impl TtyKind {
    /// Probe an arbitrary stream (anything implementing [`IsTerminal`], e.g.
    /// [`std::io::Stdout`], [`std::io::Stdin`], or a [`std::fs::File`]).
    pub fn of(stream: &impl IsTerminal) -> Self {
        if stream.is_terminal() {
            Self::Interactive
        } else {
            Self::NonInteractive
        }
    }

    /// Probe the process's standard output.
    pub fn stdout() -> Self {
        Self::of(&std::io::stdout())
    }

    /// Probe the process's standard input.
    pub fn stdin() -> Self {
        Self::of(&std::io::stdin())
    }

    /// Probe the process's standard error.
    pub fn stderr() -> Self {
        Self::of(&std::io::stderr())
    }

    /// `true` for [`TtyKind::Interactive`].
    pub fn is_interactive(self) -> bool {
        matches!(self, Self::Interactive)
    }
}

/// Whether the terminal can host an alternate screen (full-screen TUI mode).
///
/// This is a **`$TERM`-based heuristic**, not a guarantee: any interactive,
/// non-`dumb`, identified terminal is reported [`Supported`](Self::Supported).
/// A few legacy `$TERM` values (e.g. `vt52`) report `Supported` without truly
/// implementing the `\e[?1049h` alternate-screen sequence. Treat it as "very
/// likely" and fall back gracefully if entering the alternate screen fails.
///
/// # Stability
///
/// **experimental** (D-TUIN-005). Heuristic with no consumer yet; depend on it
/// at your own risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AltScreenSupport {
    /// An alternate screen can very likely be entered (heuristic).
    Supported,
    /// No alternate screen — non-TTY, or a `dumb`/unidentified terminal.
    Unsupported,
}

impl AltScreenSupport {
    /// Pure decision from explicit inputs.
    ///
    /// A non-TTY can never host an alternate screen; a `dumb` terminal or one
    /// with no `$TERM` at all is treated as incapable.
    pub fn resolve(is_tty: bool, term: Option<&str>) -> Self {
        match (is_tty, term) {
            (true, Some(t)) if !t.is_empty() && t != "dumb" => Self::Supported,
            _ => Self::Unsupported,
        }
    }

    /// Probe the live environment against standard output.
    pub fn detect() -> Self {
        let term = std::env::var("TERM").ok();
        Self::resolve(std::io::stdout().is_terminal(), term.as_deref())
    }

    /// `true` for [`AltScreenSupport::Supported`].
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

/// Detected colour capability, ordered coarsest (`None`) to richest
/// (`TrueColor`) — the [`Ord`] derive lets callers compare with `>=`.
///
/// `#[non_exhaustive]` because this models a spectrum: future terminals may
/// warrant new tiers, and the marker lets us add them without a semver break.
/// Downstream `match`es must carry a wildcard arm.
///
/// Scope note: detection is **TTY-gated** — a non-TTY (piped) stream resolves to
/// [`None`](Self::None). The `FORCE_COLOR` override (colour on a non-TTY, as some
/// CI systems request) is intentionally **not** honoured in this iteration; it is
/// a deliberate follow-up so its level semantics (`0`=off, `1/2/3`=tiers) land
/// with a real consumer rather than ahead of one.
///
/// # Stability
///
/// **experimental** (D-TUIN-005). No consumer yet and the `FORCE_COLOR` gap is
/// deferred; the grade and tiers may change once a consumer lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ColourDepth {
    /// No colour: `NO_COLOR` present, a non-TTY, or a `dumb`/unidentified terminal.
    None,
    /// Standard 16-colour ANSI.
    Ansi16,
    /// 256-colour palette (`*-256color`).
    Ansi256,
    /// 24-bit truecolour (`COLORTERM` = `truecolor`/`24bit`).
    TrueColor,
}

impl ColourDepth {
    /// Pure decision from explicit inputs, in precedence order:
    ///
    /// 1. `no_color` (the [NO_COLOR](https://no-color.org/) convention — pass the
    ///    variable's *presence*, e.g. `env::var_os("NO_COLOR").is_some()`, since
    ///    an empty value still disables colour) or a non-TTY ⇒
    ///    [`ColourDepth::None`].
    /// 2. `COLORTERM` equal (case-insensitive) to the two canonical
    ///    [termstandard](https://github.com/termstandard/colors) values
    ///    `truecolor`/`24bit` ⇒ [`ColourDepth::TrueColor`].
    /// 3. `$TERM` containing `256color` ⇒ [`ColourDepth::Ansi256`].
    /// 4. `dumb`/empty/absent `$TERM` ⇒ [`ColourDepth::None`].
    /// 5. any other identified `$TERM` ⇒ [`ColourDepth::Ansi16`].
    pub fn resolve(
        is_tty: bool,
        no_color: bool,
        colorterm: Option<&str>,
        term: Option<&str>,
    ) -> Self {
        if no_color || !is_tty {
            return Self::None;
        }
        if let Some(ct) = colorterm {
            // Exact (case-insensitive) match on the two canonical values, not a
            // substring test — `COLORTERM=24bitmap` must not read as truecolour.
            if ct.eq_ignore_ascii_case("truecolor") || ct.eq_ignore_ascii_case("24bit") {
                return Self::TrueColor;
            }
        }
        match term {
            Some(t) if t.contains("256color") => Self::Ansi256,
            None | Some("dumb" | "") => Self::None,
            Some(_) => Self::Ansi16,
        }
    }

    /// Probe the live environment against standard output.
    pub fn detect() -> Self {
        let colorterm = std::env::var("COLORTERM").ok();
        let term = std::env::var("TERM").ok();
        Self::resolve(
            std::io::stdout().is_terminal(),
            std::env::var_os("NO_COLOR").is_some(),
            colorterm.as_deref(),
            term.as_deref(),
        )
    }

    /// `true` if any colour at all is available (≥ [`ColourDepth::Ansi16`]).
    pub fn has_colour(self) -> bool {
        self >= Self::Ansi16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_depth_no_color_overrides_everything() {
        // NO_COLOR wins even with a truecolor-capable terminal.
        assert_eq!(
            ColourDepth::resolve(true, true, Some("truecolor"), Some("xterm-256color")),
            ColourDepth::None
        );
    }

    #[test]
    fn colour_depth_non_tty_is_none() {
        assert_eq!(
            ColourDepth::resolve(false, false, Some("truecolor"), Some("xterm-256color")),
            ColourDepth::None
        );
    }

    #[test]
    fn colour_depth_truecolor_from_colorterm() {
        assert_eq!(
            ColourDepth::resolve(true, false, Some("truecolor"), Some("xterm")),
            ColourDepth::TrueColor
        );
        assert_eq!(
            ColourDepth::resolve(true, false, Some("24bit"), Some("xterm")),
            ColourDepth::TrueColor
        );
        // Case-insensitive.
        assert_eq!(
            ColourDepth::resolve(true, false, Some("TrueColor"), None),
            ColourDepth::TrueColor
        );
    }

    #[test]
    fn colour_depth_colorterm_is_exact_not_substring() {
        // Non-canonical values that merely *contain* the tokens must NOT be read
        // as truecolour; they fall through to the $TERM classification.
        assert_eq!(
            ColourDepth::resolve(true, false, Some("24bitmap"), Some("xterm")),
            ColourDepth::Ansi16
        );
        assert_eq!(
            ColourDepth::resolve(
                true,
                false,
                Some("not-truecolor-yet"),
                Some("xterm-256color")
            ),
            ColourDepth::Ansi256
        );
        // An uninformative COLORTERM with a dumb terminal stays None.
        assert_eq!(
            ColourDepth::resolve(true, false, Some("1"), Some("dumb")),
            ColourDepth::None
        );
    }

    #[test]
    fn colour_depth_256_from_term() {
        assert_eq!(
            ColourDepth::resolve(true, false, None, Some("xterm-256color")),
            ColourDepth::Ansi256
        );
        assert_eq!(
            ColourDepth::resolve(true, false, None, Some("screen-256color")),
            ColourDepth::Ansi256
        );
    }

    #[test]
    fn colour_depth_ansi16_from_plain_term() {
        assert_eq!(
            ColourDepth::resolve(true, false, None, Some("xterm")),
            ColourDepth::Ansi16
        );
        assert_eq!(
            ColourDepth::resolve(true, false, Some("ansi"), Some("vt100")),
            ColourDepth::Ansi16
        );
    }

    #[test]
    fn colour_depth_dumb_or_absent_term_is_none() {
        assert_eq!(
            ColourDepth::resolve(true, false, None, Some("dumb")),
            ColourDepth::None
        );
        assert_eq!(
            ColourDepth::resolve(true, false, None, None),
            ColourDepth::None
        );
        assert_eq!(
            ColourDepth::resolve(true, false, None, Some("")),
            ColourDepth::None
        );
    }

    #[test]
    fn colour_depth_ordering_and_has_colour() {
        assert!(ColourDepth::None < ColourDepth::Ansi16);
        assert!(ColourDepth::Ansi16 < ColourDepth::Ansi256);
        assert!(ColourDepth::Ansi256 < ColourDepth::TrueColor);
        assert!(!ColourDepth::None.has_colour());
        assert!(ColourDepth::Ansi16.has_colour());
        assert!(ColourDepth::TrueColor.has_colour());
    }

    #[test]
    fn alt_screen_support_matrix() {
        assert_eq!(
            AltScreenSupport::resolve(true, Some("xterm-256color")),
            AltScreenSupport::Supported
        );
        assert_eq!(
            AltScreenSupport::resolve(true, Some("dumb")),
            AltScreenSupport::Unsupported
        );
        assert_eq!(
            AltScreenSupport::resolve(true, None),
            AltScreenSupport::Unsupported
        );
        assert_eq!(
            AltScreenSupport::resolve(true, Some("")),
            AltScreenSupport::Unsupported
        );
        assert_eq!(
            AltScreenSupport::resolve(false, Some("xterm-256color")),
            AltScreenSupport::Unsupported
        );
        assert!(AltScreenSupport::resolve(true, Some("screen")).is_supported());
    }

    #[cfg(unix)]
    #[test]
    fn tty_kind_of_non_terminal_stream() {
        // A regular file is never an interactive terminal. `IsTerminal` is only
        // implemented for real handles (not in-memory buffers), so a `/dev/null`
        // handle is the portable-on-unix way to assert the false → NonInteractive
        // mapping deterministically.
        let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
        assert_eq!(TtyKind::of(&dev_null), TtyKind::NonInteractive);
        assert!(!TtyKind::of(&dev_null).is_interactive());
    }
}

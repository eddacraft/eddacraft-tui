//! anvil-branded shell chrome with correct binary version.
//!
//! Thin wrapper over `eddacraft_tui::shell::render_shell` that pins the
//! brand to anvil-on-EddaCraft and the version to this binary's own
//! `CARGO_PKG_VERSION`. Surfaces call `render_shell` here rather than the
//! library directly so brand/version don't have to be re-passed at every
//! call site.

use eddacraft_tui::shell::{ShellBranding, render_shell as lib_render_shell};
use eddacraft_tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

/// The version rendered in the shell watermark.
///
/// Production uses the binary's own `CARGO_PKG_VERSION`. **Test builds use a
/// fixed placeholder** so the committed shell snapshots stay version-agnostic
/// across release bumps (CIB-020): a `chore(release): prepare vX` bump must not
/// require re-accepting every shell snapshot. `production_watermark_uses_cargo_pkg_version`
/// below guards both sides of the seam — it asserts the real
/// `CARGO_PKG_VERSION` (what the `cfg(not(test))` arm embeds) is well-formed
/// and pins the placeholder to `X.Y.Z`. It does not render the production
/// version through the chrome; the one-line `cfg(not(test))` wiring is correct
/// by inspection, and the render path is exercised — with the placeholder — by
/// the snapshot tests, which therefore read `vX.Y.Z`.
///
/// The placeholder is intentionally shorter than a real semver, so these
/// snapshots do not exercise the footer's width-dependent padding/truncation.
/// That layout path lives in the library (`eddacraft_tui::shell` computes
/// `watermark.width()`) and is covered there by `uses_passed_version_in_footer`,
/// which passes a realistic-width version — so no anvil-tui-side coverage is
/// lost by pinning the placeholder.
#[cfg(not(test))]
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(test)]
const VERSION: &str = "X.Y.Z";

/// Horizontal gutter between shell chrome and surface content, in cells.
/// Surfaces use this via `inset_content` so they don't hug the left/right
/// edges of the terminal.
pub const OUTER_H_MARGIN: u16 = 2;
/// Rows of breathing room above surface content under the shell header.
pub const OUTER_TOP_MARGIN: u16 = 1;

/// Carve a padded sub-rect out of the shell content area so a surface
/// doesn't hug the top-left corner. Degrades gracefully on narrow/short
/// terminals by dropping margins when there isn't room.
///
/// Every onboarding / tutorial surface should route its incoming `area`
/// through this helper so the outer breathing room is consistent across
/// the first-run flow.
#[must_use]
pub fn inset_content(area: Rect) -> Rect {
    let h = if area.width > OUTER_H_MARGIN * 2 {
        OUTER_H_MARGIN
    } else {
        0
    };
    let t = if area.height > OUTER_TOP_MARGIN {
        OUTER_TOP_MARGIN
    } else {
        0
    };
    let inner = Layout::horizontal([
        Constraint::Length(h),
        Constraint::Min(0),
        Constraint::Length(h),
    ])
    .split(area)[1];
    Layout::vertical([Constraint::Length(t), Constraint::Min(0)]).split(inner)[1]
}

/// Render the anvil-branded shell chrome around a surface content area.
///
/// Returns the inner `Rect` that the surface should render into.
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    surface_name: &str,
    help_text: &str,
    theme: &impl Theme,
) -> Rect {
    lib_render_shell(
        frame,
        area,
        ShellBranding::Anvil,
        "anvil",
        surface_name,
        help_text,
        theme,
        VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::snapshot::buffer_to_string;
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(frame, frame.area(), "Watch", "j/k navigate  q quit", &theme);
            })
            .unwrap();
    }

    #[test]
    fn returns_inner_area() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        let mut inner = Rect::default();
        terminal
            .draw(|frame| {
                inner = render_shell(frame, frame.area(), "Audit", "h/l panels  q quit", &theme);
            })
            .unwrap();

        assert_eq!(inner.height, 22);
        assert_eq!(inner.width, 80);
        assert_eq!(inner.y, 1);
    }

    #[test]
    fn snapshot_shell_chrome() {
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(
                    frame,
                    frame.area(),
                    "Gate",
                    "j/k navigate  enter expand  q quit",
                    &theme,
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render_shell(frame, frame.area(), "Init", "q quit", &theme);
            })
            .unwrap();
    }

    #[test]
    fn production_watermark_uses_cargo_pkg_version() {
        // Production renders `CARGO_PKG_VERSION`; snapshot tests render the
        // `VERSION` placeholder so snapshots don't churn on release bumps
        // (CIB-020). This test runs under `cfg(test)`, so it does NOT render
        // the production version through the chrome — it guards the string the
        // production arm embeds and pins the placeholder the test arm renders.

        // 1. The version string the `cfg(not(test))` arm embeds is well-formed.
        let real = format!("v{}", env!("CARGO_PKG_VERSION"));
        let after_v = &real[1..];
        let leading_digit = after_v.chars().next().is_some_and(|c| c.is_ascii_digit());
        assert!(
            real.starts_with('v') && leading_digit && after_v.contains('.'),
            "expected `v<major>.<minor>…` shape, got: {real}"
        );

        // 2. The snapshot placeholder is the deliberate, non-numeric marker — so
        //    a snapshot showing `vX.Y.Z` is intentional, not a stale real
        //    version. Keeping it visually distinct from any semver is the whole
        //    point of option 2; pin it so a change forces re-accepting snapshots.
        assert_eq!(
            VERSION, "X.Y.Z",
            "snapshot watermark placeholder changed; re-accept the committed snapshots"
        );
    }
}

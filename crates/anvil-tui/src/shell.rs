//! Anvil-branded shell chrome with correct binary version.
//!
//! Thin wrapper over `eddacraft_tui::shell::render_shell` that pins the
//! brand to Anvil-on-EddaCraft and the version to this binary's own
//! `CARGO_PKG_VERSION`. Surfaces call `render_shell` here rather than the
//! library directly so brand/version don't have to be re-passed at every
//! call site.

use eddacraft_tui::shell::{ShellBranding, render_shell as lib_render_shell};
use eddacraft_tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

/// The binary's own version, from the workspace `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Render the Anvil-branded shell chrome around a surface content area.
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
        ShellBranding::EddaCraft,
        "Anvil",
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
    fn version_matches_workspace() {
        let watermark = format!("v{VERSION}");
        assert!(
            watermark.starts_with('v'),
            "watermark should start with 'v': {watermark}"
        );
        let after_v = &watermark[1..];
        let leading_digit = after_v.chars().next().is_some_and(|c| c.is_ascii_digit());
        assert!(
            leading_digit && after_v.contains('.'),
            "expected `v<major>.<minor>…` shape, got: {watermark}"
        );
    }
}

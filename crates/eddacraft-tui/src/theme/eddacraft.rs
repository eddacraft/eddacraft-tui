use ratatui::style::Color;

use super::traits::Theme;

/// The `EddaCraft` Terminal Standard palette.
///
/// Colour names follow the brand design system:
/// - The Void (bg), Structure (border)
/// - Off-White (fg), Ghost Grey (muted)
/// - Anvil Ember (accent), Edda Growth (success)
/// - Brick Red (error), Dull Amber (warning)
pub struct EddaCraftTheme;

const VOID: Color = Color::Rgb(13, 13, 15);
const STRUCTURE: Color = Color::Rgb(42, 42, 46);
const OFF_WHITE: Color = Color::Rgb(235, 235, 235);
const GHOST_GREY: Color = Color::Rgb(133, 133, 138);
const ANVIL_EMBER: Color = Color::Rgb(204, 85, 0);
const EDDA_GROWTH: Color = Color::Rgb(46, 139, 87);
const BRICK_RED: Color = Color::Rgb(201, 74, 74);
const DULL_AMBER: Color = Color::Rgb(208, 140, 56);

impl Theme for EddaCraftTheme {
    fn bg(&self) -> Color {
        VOID
    }

    fn fg(&self) -> Color {
        OFF_WHITE
    }

    fn accent(&self) -> Color {
        ANVIL_EMBER
    }

    fn success(&self) -> Color {
        EDDA_GROWTH
    }

    fn error(&self) -> Color {
        BRICK_RED
    }

    fn warning(&self) -> Color {
        DULL_AMBER
    }

    fn muted(&self) -> Color {
        GHOST_GREY
    }

    fn border(&self) -> Color {
        STRUCTURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colours_are_distinct() {
        let theme = EddaCraftTheme;
        let colours = [
            theme.bg(),
            theme.fg(),
            theme.accent(),
            theme.success(),
            theme.error(),
            theme.warning(),
            theme.muted(),
            theme.border(),
        ];

        for (i, a) in colours.iter().enumerate() {
            for (j, b) in colours.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "colour {i} and {j} should be distinct");
                }
            }
        }
    }

    #[test]
    fn base_style_uses_fg_and_bg() {
        let theme = EddaCraftTheme;
        let style = theme.base();
        assert_eq!(style.fg, Some(OFF_WHITE));
        assert_eq!(style.bg, Some(VOID));
    }
}

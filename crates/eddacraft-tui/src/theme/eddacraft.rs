use ratatui::style::Color;

use super::traits::Theme;

pub struct EddaCraftTheme;

const SLATE_900: Color = Color::Rgb(15, 23, 42);
const SLATE_400: Color = Color::Rgb(148, 163, 184);
const SLATE_100: Color = Color::Rgb(241, 245, 249);
const CYAN_400: Color = Color::Rgb(34, 211, 238);
const GREEN_400: Color = Color::Rgb(74, 222, 128);
const RED_400: Color = Color::Rgb(248, 113, 113);
const AMBER_400: Color = Color::Rgb(251, 191, 36);

impl Theme for EddaCraftTheme {
    fn bg(&self) -> Color {
        SLATE_900
    }

    fn fg(&self) -> Color {
        SLATE_100
    }

    fn accent(&self) -> Color {
        CYAN_400
    }

    fn success(&self) -> Color {
        GREEN_400
    }

    fn error(&self) -> Color {
        RED_400
    }

    fn warning(&self) -> Color {
        AMBER_400
    }

    fn muted(&self) -> Color {
        SLATE_400
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
        assert_eq!(style.fg, Some(SLATE_100));
        assert_eq!(style.bg, Some(SLATE_900));
    }
}

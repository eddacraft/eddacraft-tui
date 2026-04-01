use ratatui::style::{Color, Modifier, Style};

pub trait Theme {
    fn bg(&self) -> Color;
    fn fg(&self) -> Color;
    fn accent(&self) -> Color;
    fn success(&self) -> Color;
    fn error(&self) -> Color;
    fn warning(&self) -> Color;
    fn muted(&self) -> Color;
    fn border(&self) -> Color;

    fn base(&self) -> Style {
        Style::default().fg(self.fg()).bg(self.bg())
    }

    fn highlighted(&self) -> Style {
        Style::default()
            .fg(self.bg())
            .bg(self.accent())
            .add_modifier(Modifier::BOLD)
    }

    fn title(&self) -> Style {
        Style::default()
            .fg(self.accent())
            .add_modifier(Modifier::BOLD)
    }

    fn border_focused(&self) -> Style {
        Style::default().fg(self.accent())
    }

    fn border_unfocused(&self) -> Style {
        Style::default().fg(self.border())
    }

    fn status_ok(&self) -> Style {
        Style::default()
            .fg(self.success())
            .add_modifier(Modifier::BOLD)
    }

    fn status_error(&self) -> Style {
        Style::default()
            .fg(self.error())
            .add_modifier(Modifier::BOLD)
    }

    fn status_warning(&self) -> Style {
        Style::default()
            .fg(self.warning())
            .add_modifier(Modifier::BOLD)
    }

    fn disabled(&self) -> Style {
        Style::default().fg(self.muted())
    }
}

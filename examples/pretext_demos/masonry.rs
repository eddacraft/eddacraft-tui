use eddacraft_tui::widgets::pretext::{PretextState, PretextWidget};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::time::Instant;

const PANEL_TEXTS: &[(&str, &str)] = &[
    (
        "System Status",
        "All services operational. CPU utilization at 42%. Memory usage stable \
         at 3.2GB / 8GB. Network throughput nominal. Last deployment: 2 minutes ago. \
         Active connections: 1,247. Queue depth: 0.",
    ),
    (
        "Agent Log",
        "Analyzing codebase structure... Found 47 source files across 12 modules. \
         Running type checker... 3 warnings detected. Applying suggested fix to \
         src/parser.rs:142. Rerunning tests... All 238 tests passing. Generating \
         summary report.",
    ),
    (
        "Build Output",
        "Compiling eddacraft-tui (pretext engine) \
         Compiling unicode-width v0.2.0 \
         Compiling textwrap v0.16.0 \
         Compiling demo host \
         Finished dev [unoptimized + debuginfo] target(s) in 2.34s",
    ),
    (
        "Chat",
        "User: Can you help me refactor the layout engine? \
         Assistant: Of course! I see the layout module has three main components. \
         Let me analyze the dependencies first and suggest a clean separation. \
         The key insight is that measurement and positioning should be independent phases.",
    ),
];

pub struct MasonryDemo {
    panels: Vec<PretextState>,
    panel_titles: Vec<&'static str>,
    pub layout_time_us: u128,
}

impl MasonryDemo {
    pub fn new() -> Self {
        let mut panels = Vec::new();
        let mut panel_titles = Vec::new();

        for (title, text) in PANEL_TEXTS {
            panels.push(PretextState::new(text));
            panel_titles.push(*title);
        }

        Self {
            panels,
            panel_titles,
            layout_time_us: 0,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area);

        let main = chunks[0];

        // 2x2 grid layout
        let rows =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(main);
        let top_cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        let bot_cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        let panel_areas = [top_cols[0], top_cols[1], bot_cols[0], bot_cols[1]];

        let colors = [Color::Cyan, Color::Green, Color::Yellow, Color::Magenta];

        let start = Instant::now();

        for (i, panel_area) in panel_areas.iter().enumerate() {
            if i >= self.panels.len() {
                break;
            }

            let block = Block::default()
                .title(format!(" {} ", self.panel_titles[i]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors[i]));

            let inner = block.inner(*panel_area);
            frame.render_widget(block, *panel_area);

            let widget = PretextWidget::new().base_style(Style::default().fg(Color::White));
            frame.render_stateful_widget(widget, inner, &mut self.panels[i]);
        }

        self.layout_time_us = start.elapsed().as_micros();

        // Status bar
        let total_words: usize = self.panels.iter().map(|p| p.prepared().word_count()).sum();
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" Panels: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", self.panels.len()),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Total words: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", total_words),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled("Layout (all): ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}μs", self.layout_time_us),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled(
                "[Resize terminal to see reflow]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(status, chunks[1]);
    }
}

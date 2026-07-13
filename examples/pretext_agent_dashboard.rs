//! Agent Dashboard — public showcase for `eddacraft-tui`'s `pretext` engine.
//!
//! Simulates an AI agent interface with:
//! - Streaming styled output (left panel) with per-token colours
//! - A spinning "thinking" indicator that text flows around in real time
//! - A multi-panel sidebar that reflows cleanly on resize
//! - Live performance metrics for prepare vs layout cost
//!
//! ```text
//! cargo run -p eddacraft-tui --example pretext_agent_dashboard
//! ```
// Showcase demo prioritises readability over pedantic clippy nits.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::needless_pass_by_ref_mut,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnested_or_patterns
)]

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::ExecutableCommand;
use eddacraft_tui::pretext::ExclusionZone;
use eddacraft_tui::widgets::pretext::{PretextState, PretextWidget};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io::{self, stdout};
use std::time::{Duration, Instant};

// ─── Styles ──────────────────────────────────────────────────────────────────

fn s_heading() -> Style {
    Style::default()
        .fg(Color::Rgb(129, 199, 245))
        .add_modifier(Modifier::BOLD)
}
fn s_prose() -> Style {
    Style::default().fg(Color::Rgb(204, 204, 204))
}
fn s_emphasis() -> Style {
    Style::default()
        .fg(Color::Rgb(180, 230, 160))
        .add_modifier(Modifier::ITALIC)
}
fn s_code() -> Style {
    Style::default().fg(Color::Rgb(245, 169, 127))
}
fn s_keyword() -> Style {
    Style::default()
        .fg(Color::Rgb(198, 160, 246))
        .add_modifier(Modifier::BOLD)
}
fn s_dim() -> Style {
    Style::default().fg(Color::Rgb(100, 100, 100))
}
fn s_metric_label() -> Style {
    Style::default().fg(Color::Rgb(120, 120, 140))
}
fn s_metric_value() -> Style {
    Style::default().fg(Color::Rgb(245, 169, 127))
}

// ─── Token stream ────────────────────────────────────────────────────────────

struct Token {
    text: &'static str,
    style: Style,
}

fn agent_response_tokens() -> Vec<Token> {
    vec![
        // Turn header
        Token {
            text: "Analysis ",
            style: s_heading(),
        },
        Token {
            text: "Complete",
            style: s_heading(),
        },
        Token {
            text: "\n\n",
            style: s_prose(),
        },
        // Paragraph 1
        Token {
            text: "I've ",
            style: s_prose(),
        },
        Token {
            text: "reviewed ",
            style: s_prose(),
        },
        Token {
            text: "the ",
            style: s_prose(),
        },
        Token {
            text: "codebase ",
            style: s_prose(),
        },
        Token {
            text: "and ",
            style: s_prose(),
        },
        Token {
            text: "found ",
            style: s_prose(),
        },
        Token {
            text: "three ",
            style: s_emphasis(),
        },
        Token {
            text: "key ",
            style: s_emphasis(),
        },
        Token {
            text: "optimization ",
            style: s_emphasis(),
        },
        Token {
            text: "opportunities. ",
            style: s_emphasis(),
        },
        Token {
            text: "The ",
            style: s_prose(),
        },
        Token {
            text: "layout ",
            style: s_prose(),
        },
        Token {
            text: "engine ",
            style: s_prose(),
        },
        Token {
            text: "currently ",
            style: s_prose(),
        },
        Token {
            text: "re-measures ",
            style: s_prose(),
        },
        Token {
            text: "every ",
            style: s_prose(),
        },
        Token {
            text: "frame, ",
            style: s_prose(),
        },
        Token {
            text: "which ",
            style: s_prose(),
        },
        Token {
            text: "is ",
            style: s_prose(),
        },
        Token {
            text: "the ",
            style: s_prose(),
        },
        Token {
            text: "primary ",
            style: s_prose(),
        },
        Token {
            text: "bottleneck. ",
            style: s_prose(),
        },
        Token {
            text: "\n\n",
            style: s_prose(),
        },
        // Code section
        Token {
            text: "The ",
            style: s_prose(),
        },
        Token {
            text: "fix ",
            style: s_prose(),
        },
        Token {
            text: "is ",
            style: s_prose(),
        },
        Token {
            text: "straightforward: ",
            style: s_prose(),
        },
        Token {
            text: "use ",
            style: s_prose(),
        },
        Token {
            text: "PreparedText ",
            style: s_code(),
        },
        Token {
            text: "to ",
            style: s_prose(),
        },
        Token {
            text: "cache ",
            style: s_keyword(),
        },
        Token {
            text: "measurements ",
            style: s_prose(),
        },
        Token {
            text: "in ",
            style: s_prose(),
        },
        Token {
            text: "the ",
            style: s_prose(),
        },
        Token {
            text: "prepare ",
            style: s_keyword(),
        },
        Token {
            text: "phase, ",
            style: s_prose(),
        },
        Token {
            text: "then ",
            style: s_prose(),
        },
        Token {
            text: "call ",
            style: s_prose(),
        },
        Token {
            text: "layout() ",
            style: s_code(),
        },
        Token {
            text: "with ",
            style: s_prose(),
        },
        Token {
            text: "pure ",
            style: s_prose(),
        },
        Token {
            text: "arithmetic ",
            style: s_prose(),
        },
        Token {
            text: "on ",
            style: s_prose(),
        },
        Token {
            text: "subsequent ",
            style: s_prose(),
        },
        Token {
            text: "frames. ",
            style: s_prose(),
        },
        Token {
            text: "\n\n",
            style: s_prose(),
        },
        // Results section
        Token {
            text: "Results: ",
            style: s_heading(),
        },
        Token {
            text: "\n\n",
            style: s_prose(),
        },
        Token {
            text: "Measurement ",
            style: s_prose(),
        },
        Token {
            text: "is ",
            style: s_prose(),
        },
        Token {
            text: "now ",
            style: s_prose(),
        },
        Token {
            text: "O(1) ",
            style: s_emphasis(),
        },
        Token {
            text: "amortized ",
            style: s_prose(),
        },
        Token {
            text: "for ",
            style: s_prose(),
        },
        Token {
            text: "streaming ",
            style: s_prose(),
        },
        Token {
            text: "append. ",
            style: s_prose(),
        },
        Token {
            text: "Layout ",
            style: s_prose(),
        },
        Token {
            text: "recomputes ",
            style: s_prose(),
        },
        Token {
            text: "in ",
            style: s_prose(),
        },
        Token {
            text: "under ",
            style: s_prose(),
        },
        Token {
            text: "50μs ",
            style: s_emphasis(),
        },
        Token {
            text: "for ",
            style: s_prose(),
        },
        Token {
            text: "typical ",
            style: s_prose(),
        },
        Token {
            text: "content. ",
            style: s_prose(),
        },
        Token {
            text: "Unicode ",
            style: s_prose(),
        },
        Token {
            text: "characters ",
            style: s_prose(),
        },
        Token {
            text: "like ",
            style: s_prose(),
        },
        Token {
            text: "你好世界 ",
            style: s_emphasis(),
        },
        Token {
            text: "and ",
            style: s_prose(),
        },
        Token {
            text: "symbols ",
            style: s_prose(),
        },
        Token {
            text: "are ",
            style: s_prose(),
        },
        Token {
            text: "correctly ",
            style: s_prose(),
        },
        Token {
            text: "handled ",
            style: s_prose(),
        },
        Token {
            text: "as ",
            style: s_prose(),
        },
        Token {
            text: "double-width. ",
            style: s_prose(),
        },
        Token {
            text: "\n\n",
            style: s_prose(),
        },
        // Conclusion
        Token {
            text: "The ",
            style: s_prose(),
        },
        Token {
            text: "exclusion ",
            style: s_keyword(),
        },
        Token {
            text: "zone ",
            style: s_keyword(),
        },
        Token {
            text: "system ",
            style: s_prose(),
        },
        Token {
            text: "lets ",
            style: s_prose(),
        },
        Token {
            text: "text ",
            style: s_prose(),
        },
        Token {
            text: "flow ",
            style: s_prose(),
        },
        Token {
            text: "around ",
            style: s_prose(),
        },
        Token {
            text: "dynamic ",
            style: s_prose(),
        },
        Token {
            text: "UI ",
            style: s_prose(),
        },
        Token {
            text: "elements — ",
            style: s_prose(),
        },
        Token {
            text: "notice ",
            style: s_prose(),
        },
        Token {
            text: "how ",
            style: s_prose(),
        },
        Token {
            text: "this ",
            style: s_prose(),
        },
        Token {
            text: "text ",
            style: s_prose(),
        },
        Token {
            text: "reflows ",
            style: s_emphasis(),
        },
        Token {
            text: "around ",
            style: s_prose(),
        },
        Token {
            text: "the ",
            style: s_prose(),
        },
        Token {
            text: "spinning ",
            style: s_prose(),
        },
        Token {
            text: "indicator ",
            style: s_prose(),
        },
        Token {
            text: "without ",
            style: s_prose(),
        },
        Token {
            text: "any ",
            style: s_prose(),
        },
        Token {
            text: "re-measurement. ",
            style: s_prose(),
        },
        Token {
            text: "Every ",
            style: s_prose(),
        },
        Token {
            text: "frame ",
            style: s_prose(),
        },
        Token {
            text: "is ",
            style: s_prose(),
        },
        Token {
            text: "just ",
            style: s_prose(),
        },
        Token {
            text: "arithmetic ",
            style: s_prose(),
        },
        Token {
            text: "on ",
            style: s_prose(),
        },
        Token {
            text: "cached ",
            style: s_keyword(),
        },
        Token {
            text: "widths. ",
            style: s_prose(),
        },
    ]
}

// ─── Spinner / thinking indicator ────────────────────────────────────────────

const SPINNER_FRAMES: &[&[&str]] = &[
    &[
        "╭───────────╮",
        "│  ◐  think │",
        "│    ing... │",
        "╰───────────╯",
    ],
    &[
        "╭───────────╮",
        "│  ◓  think │",
        "│    ing... │",
        "╰───────────╯",
    ],
    &[
        "╭───────────╮",
        "│  ◑  think │",
        "│    ing... │",
        "╰───────────╯",
    ],
    &[
        "╭───────────╮",
        "│  ◒  think │",
        "│    ing... │",
        "╰───────────╯",
    ],
];

const SPINNER_W: u16 = 13;
const SPINNER_H: u16 = 4;

struct Spinner {
    frame: usize,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    last_advance: Instant,
}

impl Spinner {
    fn new() -> Self {
        Self {
            frame: 0,
            x: 2.0,
            y: 1.0,
            dx: 0.4,
            dy: 0.25,
            last_advance: Instant::now(),
        }
    }

    fn tick(&mut self, max_w: u16, max_h: u16) {
        // Bounce
        self.x += self.dx;
        self.y += self.dy;
        let mx = (max_w.saturating_sub(SPINNER_W)).max(1) as f64;
        let my = (max_h.saturating_sub(SPINNER_H)).max(1) as f64;
        if self.x <= 0.0 || self.x >= mx {
            self.dx = -self.dx;
            self.x = self.x.clamp(0.0, mx);
        }
        if self.y <= 0.0 || self.y >= my {
            self.dy = -self.dy;
            self.y = self.y.clamp(0.0, my);
        }
        // Animate spinner char
        if self.last_advance.elapsed() >= Duration::from_millis(120) {
            self.frame = (self.frame + 1) % SPINNER_FRAMES.len();
            self.last_advance = Instant::now();
        }
    }

    fn exclusion(&self) -> ExclusionZone {
        ExclusionZone::rect(self.x as u16, self.y as u16, SPINNER_W, SPINNER_H)
    }

    fn draw(&self, buf: &mut Buffer, area: Rect) {
        let sx = area.x + self.x as u16;
        let sy = area.y + self.y as u16;
        let lines = SPINNER_FRAMES[self.frame];
        let style = Style::default()
            .fg(Color::Rgb(198, 160, 246))
            .add_modifier(Modifier::BOLD);
        for (i, line) in lines.iter().enumerate() {
            let y = sy + i as u16;
            if y < area.bottom() && sx < area.right() {
                buf.set_string(sx, y, line, style);
            }
        }
    }
}

// ─── Sidebar panel content ──────────────────────────────────────────────────

fn build_sidebar_panels() -> Vec<(&'static str, PretextState, Color)> {
    let mut panels = Vec::new();

    // Files changed panel
    let mut files = PretextState::new("");
    let path_style = Style::default().fg(Color::Rgb(245, 169, 127));
    let label_style = Style::default().fg(Color::Rgb(166, 218, 149));
    let dim = Style::default().fg(Color::Rgb(100, 100, 100));
    files.append_styled("src/layout.rs ", path_style);
    files.append_styled("+42 -8 ", label_style);
    files.append_styled("| ", dim);
    files.append_styled("src/prepare.rs ", path_style);
    files.append_styled("+15 -3 ", label_style);
    files.append_styled("| ", dim);
    files.append_styled("src/segment.rs ", path_style);
    files.append_styled("+7 -1 ", label_style);
    files.append_styled("| ", dim);
    files.append_styled("tests/layout.rs ", path_style);
    files.append_styled("+28 ", label_style);
    panels.push(("Files Changed", files, Color::Rgb(245, 169, 127)));

    // Test results
    let mut tests = PretextState::new("");
    let pass = Style::default().fg(Color::Rgb(166, 218, 149));
    let num = Style::default()
        .fg(Color::Rgb(129, 199, 245))
        .add_modifier(Modifier::BOLD);
    tests.append_styled("21 ", num);
    tests.append_styled("passed ", pass);
    tests.append_styled("| ", dim);
    tests.append_styled("0 ", num);
    tests.append_styled("failed ", dim);
    tests.append_styled("| ", dim);
    tests.append_styled("segment: ", dim);
    tests.append_styled("5/5 ", pass);
    tests.append_styled("prepare: ", dim);
    tests.append_styled("8/8 ", pass);
    tests.append_styled("layout: ", dim);
    tests.append_styled("4/4 ", pass);
    tests.append_styled("exclusion: ", dim);
    tests.append_styled("4/4 ", pass);
    panels.push(("Test Results", tests, Color::Rgb(166, 218, 149)));

    // Performance
    let mut perf = PretextState::new("");
    let metric = Style::default().fg(Color::Rgb(198, 160, 246));
    perf.append_styled("prepare(): ", dim);
    perf.append_styled("12μs ", metric);
    perf.append_styled("avg | ", dim);
    perf.append_styled("layout(): ", dim);
    perf.append_styled("8μs ", metric);
    perf.append_styled("avg | ", dim);
    perf.append_styled("append(): ", dim);
    perf.append_styled("2μs ", metric);
    perf.append_styled("avg | ", dim);
    perf.append_styled("Words ", dim);
    perf.append_styled("cached: ", dim);
    perf.append_styled("847 ", metric);
    perf.append_styled("| ", dim);
    perf.append_styled("Cache ", dim);
    perf.append_styled("hits: ", dim);
    perf.append_styled("99.2% ", metric);
    panels.push(("Performance", perf, Color::Rgb(198, 160, 246)));

    panels
}

// ─── App state ───────────────────────────────────────────────────────────────

struct App {
    // Main chat panel
    chat: PretextState,
    tokens: Vec<Token>,
    token_idx: usize,
    last_token: Instant,
    token_interval: Duration,

    // Animated spinner
    spinner: Spinner,
    show_spinner: bool,

    // Sidebar panels
    sidebar: Vec<(&'static str, PretextState, Color)>,

    // Metrics
    prepare_us: u128,
    layout_us: u128,
    frame_count: u64,

    paused: bool,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        // Start with a "user message" already in the chat
        let mut chat = PretextState::new("");
        let user_label = Style::default()
            .fg(Color::Rgb(129, 199, 245))
            .add_modifier(Modifier::BOLD);
        let user_text = Style::default().fg(Color::Rgb(180, 180, 180));
        chat.append_styled("You: ", user_label);
        chat.append_styled(
            "Analyze the layout engine and suggest optimizations for streaming rendering.",
            user_text,
        );
        chat.append_styled("\n\n", s_prose());
        let agent_label = Style::default()
            .fg(Color::Rgb(166, 218, 149))
            .add_modifier(Modifier::BOLD);
        chat.append_styled("Agent: ", agent_label);

        Self {
            chat,
            tokens: agent_response_tokens(),
            token_idx: 0,
            last_token: Instant::now(),
            token_interval: Duration::from_millis(45),
            spinner: Spinner::new(),
            show_spinner: true,
            sidebar: build_sidebar_panels(),
            prepare_us: 0,
            layout_us: 0,
            frame_count: 0,
            paused: false,
            should_quit: false,
        }
    }

    fn tick(&mut self, chat_area: Rect) {
        if self.paused {
            return;
        }

        // Stream tokens
        if self.token_idx < self.tokens.len() && self.last_token.elapsed() >= self.token_interval {
            let tok = &self.tokens[self.token_idx];
            let start = Instant::now();
            self.chat.append_styled(tok.text, tok.style);
            self.prepare_us = start.elapsed().as_micros();
            self.token_idx += 1;
            self.last_token = Instant::now();
        }

        // Once streaming is done, hide spinner after a beat
        if self.token_idx >= self.tokens.len() {
            self.show_spinner = false;
        }

        // Animate spinner
        if self.show_spinner {
            let w = chat_area.width.saturating_sub(2);
            let h = chat_area.height.saturating_sub(2);
            self.spinner.tick(w, h);
            self.chat.set_exclusions(vec![self.spinner.exclusion()]);
        } else {
            self.chat.set_exclusions(vec![]);
        }

        self.frame_count += 1;
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Top title bar
        let outer = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

        // Title
        let title_line = Line::from(vec![
            Span::styled(
                " pretext-tui ",
                Style::default()
                    .fg(Color::Rgb(129, 199, 245))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "agent dashboard demo ",
                Style::default().fg(Color::Rgb(100, 100, 120)),
            ),
            Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 70))),
            Span::styled(
                "[q]uit  [space]pause  [+/-]speed  [s]pinner",
                Style::default().fg(Color::Rgb(100, 100, 120)),
            ),
        ]);
        frame.render_widget(title_line, outer[0]);

        // Main content: chat (left 65%) + sidebar (right 35%)
        let cols = Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(outer[1]);

        // ── Chat panel ──
        let chat_block = Block::default()
            .title(Span::styled(
                " Agent Response ",
                Style::default()
                    .fg(Color::Rgb(166, 218, 149))
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));

        let chat_inner = chat_block.inner(cols[0]);
        frame.render_widget(chat_block, cols[0]);

        let layout_start = Instant::now();
        let w = PretextWidget::new().base_style(s_prose());
        frame.render_stateful_widget(w, chat_inner, &mut self.chat);
        self.layout_us = layout_start.elapsed().as_micros();

        // Draw spinner on top of text
        if self.show_spinner {
            self.spinner.draw(frame.buffer_mut(), chat_inner);
        }

        // ── Sidebar ──
        let sidebar_area = cols[1];
        let n = self.sidebar.len();
        let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
        let panel_areas = Layout::vertical(constraints).split(sidebar_area);

        for (i, (title, state, color)) in self.sidebar.iter_mut().enumerate() {
            let block = Block::default()
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));

            let inner = block.inner(panel_areas[i]);
            frame.render_widget(block, panel_areas[i]);

            let w = PretextWidget::new().base_style(s_dim());
            frame.render_stateful_widget(w, inner, state);
        }

        // ── Bottom status bar ──
        let streaming_done = self.token_idx >= self.tokens.len();
        let status_spans = vec![
            Span::styled(" Tokens: ", s_metric_label()),
            Span::styled(
                format!("{}/{}", self.token_idx, self.tokens.len()),
                s_metric_value(),
            ),
            Span::styled("  Words: ", s_metric_label()),
            Span::styled(
                format!("{}", self.chat.prepared().word_count()),
                Style::default()
                    .fg(Color::Rgb(166, 218, 149))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Prepare: ", s_metric_label()),
            Span::styled(format!("{}μs", self.prepare_us), s_metric_value()),
            Span::styled("  Layout: ", s_metric_label()),
            Span::styled(format!("{}μs", self.layout_us), s_metric_value()),
            Span::styled("  Frame: ", s_metric_label()),
            Span::styled(
                format!("#{}", self.frame_count),
                Style::default().fg(Color::Rgb(100, 100, 120)),
            ),
            Span::raw("  "),
            Span::styled(
                if self.paused {
                    " PAUSED "
                } else if streaming_done {
                    " COMPLETE "
                } else {
                    " STREAMING "
                },
                Style::default()
                    .fg(Color::Rgb(30, 30, 30))
                    .bg(if self.paused {
                        Color::Rgb(237, 135, 150)
                    } else if streaming_done {
                        Color::Rgb(166, 218, 149)
                    } else {
                        Color::Rgb(129, 199, 245)
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}ms/tok", self.token_interval.as_millis()),
                s_metric_label(),
            ),
        ];

        let status = Paragraph::new(Line::from(status_spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80))),
        );
        frame.render_widget(status, outer[2]);
    }
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let res = (|| -> io::Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut app = App::new();
        let mut last_chat_area = Rect::default();

        loop {
            terminal.draw(|frame| {
                let outer = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(frame.area());
                let cols =
                    Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
                        .split(outer[1]);
                last_chat_area = cols[0];

                app.render(frame);
            })?;

            app.tick(last_chat_area);

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                        KeyCode::Char(' ') => app.paused = !app.paused,
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.token_interval = app
                                .token_interval
                                .checked_sub(Duration::from_millis(10))
                                .unwrap_or(Duration::from_millis(5));
                        }
                        KeyCode::Char('-') => {
                            app.token_interval += Duration::from_millis(10);
                        }
                        KeyCode::Char('s') => {
                            app.show_spinner = !app.show_spinner;
                            if !app.show_spinner {
                                app.chat.set_exclusions(vec![]);
                            }
                        }
                        KeyCode::Char('r') => {
                            app = App::new();
                        }
                        _ => {}
                    }
                }
            }

            if app.should_quit {
                break;
            }
        }

        Ok(())
    })();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    res
}

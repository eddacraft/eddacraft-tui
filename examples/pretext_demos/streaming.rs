use eddacraft_tui::widgets::pretext::{PretextState, PretextWidget};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::time::{Duration, Instant};

/// A token with its text and style — simulates styled streaming output
/// like an AI assistant that uses different colors for code, emphasis, etc.
struct StyledToken {
    text: &'static str,
    style: Style,
}

fn prose() -> Style {
    Style::default().fg(Color::White)
}
fn emphasis() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}
fn code() -> Style {
    Style::default().fg(Color::Green)
}
fn heading() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

fn sample_tokens() -> Vec<StyledToken> {
    vec![
        // Heading
        StyledToken {
            text: "Two-Phase ",
            style: heading(),
        },
        StyledToken {
            text: "Layout ",
            style: heading(),
        },
        StyledToken {
            text: "Engine ",
            style: heading(),
        },
        StyledToken {
            text: "\n\n",
            style: prose(),
        },
        // Paragraph 1 — prose with emphasis
        StyledToken {
            text: "The ",
            style: prose(),
        },
        StyledToken {
            text: "concept ",
            style: prose(),
        },
        StyledToken {
            text: "of ",
            style: prose(),
        },
        StyledToken {
            text: "a ",
            style: prose(),
        },
        StyledToken {
            text: "two-phase ",
            style: emphasis(),
        },
        StyledToken {
            text: "layout ",
            style: emphasis(),
        },
        StyledToken {
            text: "engine ",
            style: emphasis(),
        },
        StyledToken {
            text: "is ",
            style: prose(),
        },
        StyledToken {
            text: "surprisingly ",
            style: prose(),
        },
        StyledToken {
            text: "powerful. ",
            style: prose(),
        },
        StyledToken {
            text: "By ",
            style: prose(),
        },
        StyledToken {
            text: "separating ",
            style: prose(),
        },
        StyledToken {
            text: "measurement ",
            style: emphasis(),
        },
        StyledToken {
            text: "from ",
            style: prose(),
        },
        StyledToken {
            text: "positioning, ",
            style: emphasis(),
        },
        StyledToken {
            text: "we ",
            style: prose(),
        },
        StyledToken {
            text: "unlock ",
            style: prose(),
        },
        StyledToken {
            text: "the ",
            style: prose(),
        },
        StyledToken {
            text: "ability ",
            style: prose(),
        },
        StyledToken {
            text: "to ",
            style: prose(),
        },
        StyledToken {
            text: "re-layout ",
            style: prose(),
        },
        StyledToken {
            text: "text ",
            style: prose(),
        },
        StyledToken {
            text: "at ",
            style: prose(),
        },
        StyledToken {
            text: "60fps ",
            style: emphasis(),
        },
        StyledToken {
            text: "without ",
            style: prose(),
        },
        StyledToken {
            text: "any ",
            style: prose(),
        },
        StyledToken {
            text: "repeated ",
            style: prose(),
        },
        StyledToken {
            text: "string ",
            style: prose(),
        },
        StyledToken {
            text: "measurement. ",
            style: prose(),
        },
        StyledToken {
            text: "\n\n",
            style: prose(),
        },
        // Paragraph 2 — code references
        StyledToken {
            text: "Each ",
            style: prose(),
        },
        StyledToken {
            text: "token ",
            style: prose(),
        },
        StyledToken {
            text: "calls ",
            style: prose(),
        },
        StyledToken {
            text: "append_styled() ",
            style: code(),
        },
        StyledToken {
            text: "which ",
            style: prose(),
        },
        StyledToken {
            text: "only ",
            style: prose(),
        },
        StyledToken {
            text: "measures ",
            style: prose(),
        },
        StyledToken {
            text: "the ",
            style: prose(),
        },
        StyledToken {
            text: "new ",
            style: prose(),
        },
        StyledToken {
            text: "text. ",
            style: prose(),
        },
        StyledToken {
            text: "Existing ",
            style: prose(),
        },
        StyledToken {
            text: "cached ",
            style: prose(),
        },
        StyledToken {
            text: "widths ",
            style: prose(),
        },
        StyledToken {
            text: "are ",
            style: prose(),
        },
        StyledToken {
            text: "preserved. ",
            style: prose(),
        },
        StyledToken {
            text: "The ",
            style: prose(),
        },
        StyledToken {
            text: "layout() ",
            style: code(),
        },
        StyledToken {
            text: "phase ",
            style: prose(),
        },
        StyledToken {
            text: "runs ",
            style: prose(),
        },
        StyledToken {
            text: "pure ",
            style: prose(),
        },
        StyledToken {
            text: "arithmetic ",
            style: prose(),
        },
        StyledToken {
            text: "on ",
            style: prose(),
        },
        StyledToken {
            text: "the ",
            style: prose(),
        },
        StyledToken {
            text: "cached ",
            style: prose(),
        },
        StyledToken {
            text: "values. ",
            style: prose(),
        },
        StyledToken {
            text: "\n\n",
            style: prose(),
        },
        // Paragraph 3 — unicode showcase
        StyledToken {
            text: "Unicode ",
            style: emphasis(),
        },
        StyledToken {
            text: "support ",
            style: emphasis(),
        },
        StyledToken {
            text: "is ",
            style: prose(),
        },
        StyledToken {
            text: "built-in: ",
            style: prose(),
        },
        StyledToken {
            text: "CJK ",
            style: prose(),
        },
        StyledToken {
            text: "characters ",
            style: prose(),
        },
        StyledToken {
            text: "like ",
            style: prose(),
        },
        StyledToken {
            text: "你好 ",
            style: emphasis(),
        },
        StyledToken {
            text: "are ",
            style: prose(),
        },
        StyledToken {
            text: "correctly ",
            style: prose(),
        },
        StyledToken {
            text: "measured ",
            style: prose(),
        },
        StyledToken {
            text: "as ",
            style: prose(),
        },
        StyledToken {
            text: "double-width. ",
            style: prose(),
        },
        StyledToken {
            text: "The ",
            style: prose(),
        },
        StyledToken {
            text: "unicode-width ",
            style: code(),
        },
        StyledToken {
            text: "crate ",
            style: prose(),
        },
        StyledToken {
            text: "does ",
            style: prose(),
        },
        StyledToken {
            text: "the ",
            style: prose(),
        },
        StyledToken {
            text: "heavy ",
            style: prose(),
        },
        StyledToken {
            text: "lifting ",
            style: prose(),
        },
        StyledToken {
            text: "during ",
            style: prose(),
        },
        StyledToken {
            text: "prepare(), ",
            style: code(),
        },
        StyledToken {
            text: "so ",
            style: prose(),
        },
        StyledToken {
            text: "layout ",
            style: prose(),
        },
        StyledToken {
            text: "never ",
            style: prose(),
        },
        StyledToken {
            text: "touches ",
            style: prose(),
        },
        StyledToken {
            text: "the ",
            style: prose(),
        },
        StyledToken {
            text: "text ",
            style: prose(),
        },
        StyledToken {
            text: "again. ",
            style: prose(),
        },
    ]
}

pub struct StreamingDemo {
    pub state: PretextState,
    tokens: Vec<StyledToken>,
    token_index: usize,
    last_token_time: Instant,
    token_interval: Duration,
    pub prepare_time_us: u128,
    pub layout_time_us: u128,
    paused: bool,
}

impl StreamingDemo {
    pub fn new() -> Self {
        Self {
            state: PretextState::new(""),
            tokens: sample_tokens(),
            token_index: 0,
            last_token_time: Instant::now(),
            token_interval: Duration::from_millis(50),
            prepare_time_us: 0,
            layout_time_us: 0,
            paused: false,
        }
    }

    pub fn tick(&mut self) {
        if self.paused || self.token_index >= self.tokens.len() {
            return;
        }

        if self.last_token_time.elapsed() >= self.token_interval {
            let token = &self.tokens[self.token_index];
            let start = Instant::now();
            self.state.append_styled(token.text, token.style);
            self.prepare_time_us = start.elapsed().as_micros();

            self.token_index += 1;
            self.last_token_time = Instant::now();
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn reset(&mut self) {
        let interval = self.token_interval;
        *self = Self::new();
        self.token_interval = interval;
    }

    pub fn speed_up(&mut self) {
        self.token_interval = self
            .token_interval
            .checked_sub(Duration::from_millis(10))
            .unwrap_or(Duration::from_millis(5));
    }

    pub fn slow_down(&mut self) {
        self.token_interval += Duration::from_millis(10);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area);

        let block = Block::default()
            .title(" Streaming AI Output (per-word styling) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let start = Instant::now();
        let widget = PretextWidget::new().base_style(Style::default().fg(Color::White));
        frame.render_stateful_widget(widget, inner, &mut self.state);
        self.layout_time_us = start.elapsed().as_micros();

        // Status bar
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" Tokens: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{}", self.token_index, self.tokens.len()),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Words: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", self.state.prepared().word_count()),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled("Prepare: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}μs", self.prepare_time_us),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled("Layout: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}μs", self.layout_time_us),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled(
                if self.paused {
                    "[PAUSED]"
                } else {
                    "[STREAMING]"
                },
                Style::default()
                    .fg(if self.paused {
                        Color::Red
                    } else {
                        Color::Green
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{}ms/tok", self.token_interval.as_millis()),
                Style::default().fg(Color::DarkGray),
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

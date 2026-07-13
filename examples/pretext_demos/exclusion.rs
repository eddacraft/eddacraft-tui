use eddacraft_tui::widgets::pretext::{PretextState, PretextWidget};
use eddacraft_tui::pretext::ExclusionZone;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::time::Instant;

const FILLER_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim \
veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo \
consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum \
dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, \
sunt in culpa qui officia deserunt mollit anim id est laborum. Curabitur pretium \
tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, \
est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in \
mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec \
lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula ut dictum pharetra, \
nisi nunc fringilla magna, in commodo elit erat nec turpis. Ut pharetra augue nec augue.";

pub struct ExclusionDemo {
    pub state: PretextState,
    shape_x: f64,
    shape_y: f64,
    dx: f64,
    dy: f64,
    shape_width: u16,
    shape_height: u16,
    pub layout_time_us: u128,
    animating: bool,
}

impl ExclusionDemo {
    pub fn new() -> Self {
        let mut state = PretextState::new(FILLER_TEXT);
        state.set_exclusions(vec![ExclusionZone::rect(40, 3, 15, 8)]);

        Self {
            state,
            shape_x: 40.0,
            shape_y: 3.0,
            dx: 0.5,
            dy: 0.3,
            shape_width: 15,
            shape_height: 8,
            layout_time_us: 0,
            animating: true,
        }
    }

    pub fn tick(&mut self, area_width: u16, area_height: u16) {
        if !self.animating {
            return;
        }

        // Bounce the shape around
        self.shape_x += self.dx;
        self.shape_y += self.dy;

        let raw_max_x = area_width.saturating_sub(self.shape_width) as f64;
        let raw_max_y = area_height.saturating_sub(self.shape_height) as f64;

        let min_x = if raw_max_x >= 1.0 { 1.0 } else { 0.0 };
        let max_x = raw_max_x.max(min_x);
        let min_y = 0.0;
        let max_y = raw_max_y.max(min_y);

        if self.shape_x <= min_x || self.shape_x >= max_x {
            self.dx = -self.dx;
            self.shape_x = self.shape_x.clamp(min_x, max_x);
        }
        if self.shape_y <= min_y || self.shape_y >= max_y {
            self.dy = -self.dy;
            self.shape_y = self.shape_y.clamp(min_y, max_y);
        }

        // Update exclusion zone — this invalidates layout but NOT the prepare cache
        self.state.set_exclusions(vec![ExclusionZone::rect(
            self.shape_x as u16,
            self.shape_y as u16,
            self.shape_width,
            self.shape_height,
        )]);
    }

    pub fn toggle_animate(&mut self) {
        self.animating = !self.animating;
    }

    pub fn move_shape(&mut self, dx: i16, dy: i16) {
        self.shape_x = (self.shape_x + dx as f64).max(0.0);
        self.shape_y = (self.shape_y + dy as f64).max(0.0);
        self.state.set_exclusions(vec![ExclusionZone::rect(
            self.shape_x as u16,
            self.shape_y as u16,
            self.shape_width,
            self.shape_height,
        )]);
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area);

        let block = Block::default()
            .title(" Text Flowing Around Shape ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        // Render text layout
        let start = Instant::now();
        let widget = PretextWidget::new().base_style(Style::default().fg(Color::White));
        frame.render_stateful_widget(widget, inner, &mut self.state);
        self.layout_time_us = start.elapsed().as_micros();

        // Draw the exclusion shape on top
        let sx = self.shape_x as u16;
        let sy = self.shape_y as u16;
        draw_box(
            frame.buffer_mut(),
            Rect::new(
                inner.x + sx,
                inner.y + sy,
                self.shape_width.min(inner.width.saturating_sub(sx)),
                self.shape_height.min(inner.height.saturating_sub(sy)),
            ),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        // Status bar
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" Shape: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("({}, {})", sx, sy),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Layout: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}μs", self.layout_time_us),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled(
                if self.animating {
                    "[ANIMATING]"
                } else {
                    "[PAUSED - arrows to move]"
                },
                Style::default()
                    .fg(if self.animating {
                        Color::Green
                    } else {
                        Color::Red
                    })
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

fn draw_box(buf: &mut Buffer, area: Rect, style: Style) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    // Clamp to buffer bounds
    let buf_area = *buf.area();
    let area = area.intersection(buf_area);
    if area.is_empty() {
        return;
    }

    // Top and bottom borders
    for x in area.left()..area.right() {
        if x < buf_area.right() {
            if area.top() < buf_area.bottom() {
                buf[(x, area.top())].set_char('─').set_style(style);
            }
            let bottom = area.bottom().saturating_sub(1);
            if bottom < buf_area.bottom() {
                buf[(x, bottom)].set_char('─').set_style(style);
            }
        }
    }
    // Left and right borders
    for y in area.top()..area.bottom() {
        if y < buf_area.bottom() {
            if area.left() < buf_area.right() {
                buf[(area.left(), y)].set_char('│').set_style(style);
            }
            let right = area.right().saturating_sub(1);
            if right < buf_area.right() {
                buf[(right, y)].set_char('│').set_style(style);
            }
        }
    }
    // Corners
    let set_corner = |buf: &mut Buffer, x: u16, y: u16, ch: char| {
        if x < buf_area.right() && y < buf_area.bottom() {
            buf[(x, y)].set_char(ch).set_style(style);
        }
    };
    set_corner(buf, area.left(), area.top(), '┌');
    set_corner(buf, area.right().saturating_sub(1), area.top(), '┐');
    set_corner(buf, area.left(), area.bottom().saturating_sub(1), '└');
    set_corner(
        buf,
        area.right().saturating_sub(1),
        area.bottom().saturating_sub(1),
        '┘',
    );

    // Fill interior
    for y in (area.top() + 1)..(area.bottom().saturating_sub(1)) {
        for x in (area.left() + 1)..(area.right().saturating_sub(1)) {
            if x < buf_area.right() && y < buf_area.bottom() {
                buf[(x, y)].set_char('░').set_style(style);
            }
        }
    }
}

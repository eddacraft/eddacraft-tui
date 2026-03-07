use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, StatefulWidget, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct LogFilter {
    pub enabled_levels: HashSet<LogLevel>,
    pub search: String,
}

impl Default for LogFilter {
    fn default() -> Self {
        let enabled_levels = [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
        ]
        .into_iter()
        .collect();
        Self {
            enabled_levels,
            search: String::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct LogPanelState {
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub filter: LogFilter,
    pub search_mode: bool,
    pub search_input: String,
}

impl LogPanelState {
    pub fn toggle_level(&mut self, level: LogLevel) {
        if self.filter.enabled_levels.contains(&level) {
            self.filter.enabled_levels.remove(&level);
        } else {
            self.filter.enabled_levels.insert(level);
        }
    }

    pub fn set_search<S: Into<String>>(&mut self, search: S) {
        self.search_input = search.into();
        self.filter.search = self.search_input.clone();
    }

    pub fn scroll_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, total_entries: usize) {
        if total_entries == 0 {
            self.selected_index = 0;
            return;
        }
        self.selected_index = (self.selected_index + 1).min(total_entries - 1);
    }

    pub fn jump_to_top(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn jump_to_bottom(&mut self, total_entries: usize) {
        if total_entries == 0 {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }
        self.selected_index = total_entries - 1;
        self.scroll_offset = self.selected_index;
    }

    pub fn next_match(&mut self, entries: &[LogEntry]) -> bool {
        let matches = self.filtered_indices(entries);
        if matches.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return false;
        }

        self.selected_index = (self.selected_index + 1) % matches.len();
        true
    }

    pub fn prev_match(&mut self, entries: &[LogEntry]) -> bool {
        let matches = self.filtered_indices(entries);
        if matches.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
            return false;
        }

        self.selected_index = self
            .selected_index
            .checked_sub(1)
            .unwrap_or(matches.len() - 1);
        true
    }

    #[must_use]
    pub fn filtered_indices(&self, entries: &[LogEntry]) -> Vec<usize> {
        entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if matches_filter(entry, &self.filter) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[allow(clippy::struct_excessive_bools)]
pub struct LogPanel<'a, T: Theme> {
    entries: &'a [LogEntry],
    theme: &'a T,
    block: Option<Block<'a>>,
    max_visible: usize,
    title: &'a str,
    show_filter: bool,
    show_search: bool,
    auto_scroll: bool,
    focused: bool,
}

impl<'a, T: Theme> LogPanel<'a, T> {
    pub fn new(entries: &'a [LogEntry], theme: &'a T) -> Self {
        Self {
            entries,
            theme,
            block: None,
            max_visible: 8,
            title: "Logs",
            show_filter: true,
            show_search: true,
            auto_scroll: false,
            focused: false,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    #[must_use]
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible;
        self
    }

    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    #[must_use]
    pub fn show_filter(mut self, show_filter: bool) -> Self {
        self.show_filter = show_filter;
        self
    }

    #[must_use]
    pub fn show_search(mut self, show_search: bool) -> Self {
        self.show_search = show_search;
        self
    }

    #[must_use]
    pub fn auto_scroll(mut self, auto_scroll: bool) -> Self {
        self.auto_scroll = auto_scroll;
        self
    }

    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<T: Theme> StatefulWidget for LogPanel<'_, T> {
    type State = LogPanelState;

    #[allow(clippy::too_many_lines)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let border_type = if self.focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };
        let border_style = if self.focused {
            self.theme.border_focused()
        } else {
            self.theme.border_unfocused()
        };

        let mut block = self
            .block
            .unwrap_or_else(|| Block::default().borders(Borders::ALL));
        block = block
            .border_type(border_type)
            .border_style(border_style)
            .title(Line::styled(
                format!("{} ({})", self.title, self.entries.len()),
                self.theme.title(),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        state.filter.search = state.search_input.clone();
        let filtered_indices = state.filtered_indices(self.entries);
        let filtered_entries: Vec<&LogEntry> = filtered_indices
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .collect();

        if self.auto_scroll && !filtered_entries.is_empty() {
            state.selected_index = filtered_entries.len() - 1;
        }

        if filtered_entries.is_empty() {
            state.selected_index = 0;
            state.scroll_offset = 0;
        } else {
            state.selected_index = state.selected_index.min(filtered_entries.len() - 1);
        }

        let mut rows = Vec::new();
        if self.show_filter {
            rows.push(Constraint::Length(1));
        }
        if self.show_search {
            rows.push(Constraint::Length(1));
        }
        rows.push(Constraint::Min(1));
        rows.push(Constraint::Length(1));
        let chunks = Layout::vertical(rows).split(inner);

        let mut cursor = 0;

        if self.show_filter {
            let filter_line = render_filter_line(&state.filter, self.theme);
            filter_line.render(chunks[cursor], buf);
            cursor += 1;
        }

        if self.show_search {
            let prefix_style = if state.search_mode {
                self.theme.title()
            } else {
                self.theme.disabled()
            };
            let line = Line::from(vec![
                Span::styled("Search: ", prefix_style),
                Span::styled(
                    if state.search_input.is_empty() {
                        "(type to filter)"
                    } else {
                        &state.search_input
                    },
                    if state.search_input.is_empty() {
                        self.theme.disabled()
                    } else {
                        self.theme.base()
                    },
                ),
            ]);
            line.render(chunks[cursor], buf);
            cursor += 1;
        }

        let entries_area = chunks[cursor];
        cursor += 1;
        let help_area = chunks[cursor];

        let visible_height = usize::from(entries_area.height).min(self.max_visible.max(1));
        if visible_height > 0 && !filtered_entries.is_empty() {
            if state.selected_index < state.scroll_offset {
                state.scroll_offset = state.selected_index;
            } else if state.selected_index >= state.scroll_offset + visible_height {
                state.scroll_offset = state.selected_index - visible_height + 1;
            }
        }

        if filtered_entries.is_empty() {
            Line::styled(
                "No log entries recognised by current filter",
                self.theme.disabled(),
            )
            .render(entries_area, buf);
        } else {
            let end = (state.scroll_offset + visible_height).min(filtered_entries.len());
            for (row_index, entry) in filtered_entries[state.scroll_offset..end]
                .iter()
                .enumerate()
            {
                #[allow(clippy::cast_possible_truncation)]
                let y = entries_area.y + row_index as u16;
                let row_area = Rect::new(entries_area.x, y, entries_area.width, 1);
                let style = if state.scroll_offset + row_index == state.selected_index {
                    self.theme.highlighted()
                } else {
                    level_style(entry.level, self.theme)
                };

                let line = Line::from(vec![
                    Span::styled(level_icon(entry.level).to_string(), style),
                    Span::raw(" "),
                    Span::styled(entry.timestamp.as_str(), self.theme.disabled()),
                    Span::raw(" "),
                    Span::styled(entry.source.as_str(), self.theme.muted()),
                    Span::raw(" - "),
                    Span::styled(entry.message.as_str(), style),
                ]);
                line.render(row_area, buf);
            }
        }

        let has_more_up = state.scroll_offset > 0;
        let has_more_down = state.scroll_offset + visible_height < filtered_entries.len();
        let help = format!(
            "[j/k] scroll  [g/G] jump  [/] search  {}{}",
            if has_more_up { "^ " } else { "" },
            if has_more_down { "v" } else { "" }
        );
        Line::styled(help, self.theme.disabled()).render(help_area, buf);
    }
}

fn level_style<T: Theme>(level: LogLevel, theme: &T) -> ratatui::style::Style {
    match level {
        LogLevel::Error => ratatui::style::Style::default().fg(theme.error()),
        LogLevel::Warn => ratatui::style::Style::default().fg(theme.warning()),
        LogLevel::Info => ratatui::style::Style::default().fg(theme.muted()),
        LogLevel::Debug => theme.disabled(),
    }
}

fn level_icon(level: LogLevel) -> char {
    match level {
        LogLevel::Error => '✖',
        LogLevel::Warn => '◈',
        LogLevel::Info => '◇',
        LogLevel::Debug => '▪',
    }
}

fn matches_filter(entry: &LogEntry, filter: &LogFilter) -> bool {
    if !filter.enabled_levels.contains(&entry.level) {
        return false;
    }

    let search = filter.search.trim();
    if search.is_empty() {
        return true;
    }

    let search = search.to_ascii_lowercase();
    entry.message.to_ascii_lowercase().contains(&search)
        || entry.source.to_ascii_lowercase().contains(&search)
        || entry.timestamp.to_ascii_lowercase().contains(&search)
        || entry.id.to_ascii_lowercase().contains(&search)
}

fn render_filter_line<T: Theme>(filter: &LogFilter, theme: &T) -> Line<'static> {
    let levels = [
        ("E", LogLevel::Error),
        ("W", LogLevel::Warn),
        ("I", LogLevel::Info),
        ("D", LogLevel::Debug),
    ];

    let spans: Vec<Span<'static>> = levels
        .iter()
        .flat_map(|(label, level)| {
            let enabled = filter.enabled_levels.contains(level);
            let indicator = if enabled { "on" } else { "off" };
            let style = if enabled {
                level_style(*level, theme)
            } else {
                theme.disabled()
            };
            vec![
                Span::styled(format!("[{label}:{indicator}]"), style),
                Span::raw(" "),
            ]
        })
        .collect();

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<LogEntry> {
        vec![
            LogEntry {
                id: "1".to_string(),
                timestamp: "12:00:01".to_string(),
                level: LogLevel::Info,
                message: "initialised".to_string(),
                source: "kernel".to_string(),
            },
            LogEntry {
                id: "2".to_string(),
                timestamp: "12:00:02".to_string(),
                level: LogLevel::Error,
                message: "failed to parse".to_string(),
                source: "parser".to_string(),
            },
        ]
    }

    #[test]
    fn toggle_level_removes_and_readds_level() {
        let mut state = LogPanelState::default();
        assert!(state.filter.enabled_levels.contains(&LogLevel::Info));

        state.toggle_level(LogLevel::Info);
        assert!(!state.filter.enabled_levels.contains(&LogLevel::Info));

        state.toggle_level(LogLevel::Info);
        assert!(state.filter.enabled_levels.contains(&LogLevel::Info));
    }

    #[test]
    fn set_search_filters_entries() {
        let entries = sample_entries();
        let mut state = LogPanelState::default();

        state.set_search("parse");
        let matches = state.filtered_indices(&entries);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], 1);
    }

    #[test]
    fn scrolling_stays_within_bounds() {
        let mut state = LogPanelState::default();

        state.scroll_up();
        assert_eq!(state.selected_index, 0);

        state.scroll_down(2);
        state.scroll_down(2);
        state.scroll_down(2);
        assert_eq!(state.selected_index, 1);

        state.jump_to_bottom(2);
        assert_eq!(state.selected_index, 1);
        state.jump_to_top();
        assert_eq!(state.selected_index, 0);
    }
}

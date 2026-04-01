use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, StatefulWidget, Widget};

use crate::theme::Theme;

const FRACTION_BLOCKS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Cached,
}

#[derive(Debug, Clone)]
pub struct CheckProgress {
    pub id: String,
    pub name: String,
    pub status: CheckStatus,
    pub progress: u8,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub duration_ms: Option<u64>,
    pub message: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Default)]
pub struct ParallelProgressState {
    pub checks: Vec<CheckProgress>,
    pub start_time: Option<Instant>,
}

#[must_use]
pub fn calculate_overall_progress(checks: &[CheckProgress]) -> u8 {
    if checks.is_empty() {
        return 0;
    }

    let total: u64 = checks
        .iter()
        .map(|check| u64::from(effective_progress(check)))
        .sum();

    #[allow(clippy::cast_possible_truncation)]
    {
        (total / checks.len() as u64) as u8
    }
}

#[must_use]
pub fn calculate_eta(checks: &[CheckProgress], elapsed: Duration) -> Option<Duration> {
    let progress = calculate_overall_progress(checks);
    if progress == 0 || progress >= 100 {
        return None;
    }

    let elapsed_ms = elapsed.as_millis();
    let remaining_ratio = u128::from(100_u8.saturating_sub(progress));
    let progress_ratio = u128::from(progress);
    let remaining_ms = elapsed_ms.saturating_mul(remaining_ratio) / progress_ratio;

    #[allow(clippy::cast_possible_truncation)]
    Some(Duration::from_millis(remaining_ms as u64))
}

#[must_use]
pub fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        return format!("{duration_ms}ms");
    }

    let total_seconds = duration_ms / 1_000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    if minutes == 0 {
        format!("{seconds}s")
    } else {
        format!("{minutes}m {seconds}s")
    }
}

pub struct ParallelProgress<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    title: &'a str,
    show_eta: bool,
    show_overall: bool,
    compact: bool,
}

impl<'a, T: Theme> ParallelProgress<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            block: None,
            title: "Parallel Progress",
            show_eta: true,
            show_overall: true,
            compact: false,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    #[must_use]
    pub fn show_eta(mut self, show_eta: bool) -> Self {
        self.show_eta = show_eta;
        self
    }

    #[must_use]
    pub fn show_overall(mut self, show_overall: bool) -> Self {
        self.show_overall = show_overall;
        self
    }

    #[must_use]
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl<T: Theme> StatefulWidget for ParallelProgress<'_, T> {
    type State = ParallelProgressState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let complete = state
            .checks
            .iter()
            .filter(|check| {
                matches!(
                    check.status,
                    CheckStatus::Passed
                        | CheckStatus::Failed
                        | CheckStatus::Skipped
                        | CheckStatus::Cached
                )
            })
            .count();

        let mut block = self
            .block
            .unwrap_or_else(|| Block::default().borders(Borders::ALL));
        block = block
            .border_style(self.theme.border_focused())
            .title(Line::styled(
                format!("{} ({}/{})", self.title, complete, state.checks.len()),
                self.theme.title(),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let mut constraints = vec![Constraint::Min(1)];
        if self.show_overall {
            constraints.push(Constraint::Length(1));
        }
        if self.show_eta {
            constraints.push(Constraint::Length(1));
        }
        let chunks = Layout::vertical(constraints).split(inner);

        let checks_area = chunks[0];
        let check_rows = usize::from(checks_area.height);

        for (row_index, check) in state.checks.iter().take(check_rows).enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let y = checks_area.y + row_index as u16;
            let row_area = Rect::new(checks_area.x, y, checks_area.width, 1);
            let row_chunks = Layout::horizontal([
                Constraint::Length(14),
                Constraint::Min(8),
                Constraint::Length(9),
            ])
            .split(row_area);

            let name = truncate_name(&check.name, usize::from(row_chunks[0].width));
            Line::styled(name, self.theme.base()).render(row_chunks[0], buf);

            let status_style = status_style(check.status, self.theme);
            let status_icon = status_icon(check.status);
            let progress_text = if self.compact || !matches!(check.status, CheckStatus::Running) {
                if let Some(message) = &check.message {
                    format!("{status_icon} {message}")
                } else {
                    format!("{status_icon} {}%", effective_progress(check))
                }
            } else {
                let bar_width = usize::from(row_chunks[1].width).saturating_sub(5);
                let bar = render_fractional_bar(bar_width, effective_progress(check));
                format!("{bar} {:>3}%", effective_progress(check))
            };
            Line::styled(progress_text, status_style).render(row_chunks[1], buf);

            let duration =
                resolve_duration(check).map_or_else(|| "--".to_string(), format_duration);
            Line::styled(duration, self.theme.disabled()).render(row_chunks[2], buf);
        }

        let mut cursor = 1;
        if self.show_overall {
            let overall = calculate_overall_progress(&state.checks);
            let line = format!(
                "Overall {} {:>3}%",
                render_fractional_bar(
                    usize::from(chunks[cursor].width).saturating_sub(13),
                    overall
                ),
                overall
            );
            Line::styled(line, self.theme.base()).render(chunks[cursor], buf);
            cursor += 1;
        }

        if self.show_eta {
            let eta_line = if let Some(started) = state.start_time {
                let elapsed = Instant::now().saturating_duration_since(started);
                calculate_eta(&state.checks, elapsed).map_or_else(
                    || "ETA: --".to_string(),
                    |eta| {
                        let eta_ms = u64::try_from(eta.as_millis()).unwrap_or(u64::MAX);
                        format!("ETA: {}", format_duration(eta_ms))
                    },
                )
            } else {
                "ETA: --".to_string()
            };
            Line::styled(eta_line, self.theme.disabled()).render(chunks[cursor], buf);
        }
    }
}

fn effective_progress(check: &CheckProgress) -> u8 {
    if check.cached {
        return 100;
    }

    match check.status {
        CheckStatus::Pending => 0,
        CheckStatus::Running => check.progress.min(100),
        CheckStatus::Passed | CheckStatus::Failed | CheckStatus::Skipped | CheckStatus::Cached => {
            100
        }
    }
}

fn status_icon(status: CheckStatus) -> char {
    match status {
        CheckStatus::Passed => '◆',
        CheckStatus::Failed => '✖',
        CheckStatus::Running => '●',
        CheckStatus::Pending | CheckStatus::Skipped => '○',
        CheckStatus::Cached => '⚡',
    }
}

fn status_style<T: Theme>(status: CheckStatus, theme: &T) -> Style {
    match status {
        CheckStatus::Passed | CheckStatus::Cached => Style::default().fg(theme.success()),
        CheckStatus::Failed => Style::default().fg(theme.error()),
        CheckStatus::Running => Style::default().fg(theme.accent()),
        CheckStatus::Pending => theme.disabled(),
        CheckStatus::Skipped => Style::default().fg(theme.muted()),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn render_fractional_bar(width: usize, progress: u8) -> String {
    if width == 0 {
        return String::new();
    }

    let total_eighths = width * 8;
    let filled_eighths = (total_eighths * usize::from(progress)) / 100;
    let full_blocks = filled_eighths / 8;
    let remainder = filled_eighths % 8;

    let mut bar = String::new();
    bar.push_str(&"█".repeat(full_blocks));
    if remainder > 0 {
        bar.push(FRACTION_BLOCKS[remainder - 1]);
    }

    let used = full_blocks + usize::from(remainder > 0);
    bar.push_str(&"░".repeat(width.saturating_sub(used)));
    bar
}

fn truncate_name(name: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut output = String::new();
    for character in name.chars().take(width) {
        output.push(character);
    }

    if output.chars().count() < width {
        output.push_str(&" ".repeat(width - output.chars().count()));
    }

    output
}

#[allow(clippy::cast_possible_truncation)]
fn resolve_duration(check: &CheckProgress) -> Option<u64> {
    if let Some(duration) = check.duration_ms {
        return Some(duration);
    }

    match (check.start_time, check.end_time) {
        (Some(start), Some(end)) => Some(end.saturating_duration_since(start).as_millis() as u64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_progress_uses_weighted_status_progress() {
        let checks = vec![
            CheckProgress {
                id: "a".to_string(),
                name: "lint".to_string(),
                status: CheckStatus::Passed,
                progress: 100,
                start_time: None,
                end_time: None,
                duration_ms: Some(1_000),
                message: None,
                cached: false,
            },
            CheckProgress {
                id: "b".to_string(),
                name: "tests".to_string(),
                status: CheckStatus::Running,
                progress: 50,
                start_time: None,
                end_time: None,
                duration_ms: None,
                message: None,
                cached: false,
            },
        ];

        assert_eq!(calculate_overall_progress(&checks), 75);
    }

    #[test]
    fn eta_scales_from_elapsed_and_progress() {
        let checks = vec![CheckProgress {
            id: "a".to_string(),
            name: "lint".to_string(),
            status: CheckStatus::Running,
            progress: 50,
            start_time: None,
            end_time: None,
            duration_ms: None,
            message: None,
            cached: false,
        }];

        let eta = calculate_eta(&checks, Duration::from_secs(10));
        assert_eq!(eta, Some(Duration::from_secs(10)));
    }

    #[test]
    fn format_duration_handles_milliseconds_seconds_and_minutes() {
        assert_eq!(format_duration(512), "512ms");
        assert_eq!(format_duration(12_000), "12s");
        assert_eq!(format_duration(61_000), "1m 1s");
    }
}

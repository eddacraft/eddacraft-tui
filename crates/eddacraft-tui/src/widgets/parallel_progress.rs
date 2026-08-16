use std::fmt;
use std::time::{Duration, Instant};

use animate_core::Animate;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, StatefulWidget, Widget};
use unicode_width::UnicodeWidthChar;

use crate::theme::Theme;
use crate::widgets::spinner::SpinnerPreset;
use crate::widgets::{AnimatedU8, animated_u8};

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
#[non_exhaustive]
pub struct CheckProgress {
    pub id: String,
    pub name: String,
    pub status: CheckStatus,
    pub progress: u8,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub duration_ms: Option<u64>,
    pub message: Option<String>,
}

impl CheckProgress {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: CheckStatus::Pending,
            progress: 0,
            start_time: None,
            end_time: None,
            duration_ms: None,
            message: None,
        }
    }
}

#[non_exhaustive]
pub struct ParallelProgressState {
    pub checks: Vec<CheckProgress>,
    pub start_time: Option<Instant>,
    anim_overall: AnimatedU8,
    anim_overall_target: u8,
}

impl Default for ParallelProgressState {
    fn default() -> Self {
        Self {
            checks: Vec::new(),
            start_time: None,
            anim_overall: animated_u8(0),
            anim_overall_target: 0,
        }
    }
}

impl fmt::Debug for ParallelProgressState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParallelProgressState")
            .field("checks", &self.checks)
            .field("start_time", &self.start_time)
            .finish_non_exhaustive()
    }
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

    #[allow(clippy::too_many_lines)]
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
            let y = checks_area.y.saturating_add(row_index as u16);
            if y >= checks_area.y.saturating_add(checks_area.height) {
                break;
            }
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
            let status_icon = status_icon(check);
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
            let raw_overall = calculate_overall_progress(&state.checks);
            if raw_overall != state.anim_overall_target {
                state.anim_overall.set(raw_overall);
                state.anim_overall_target = raw_overall;
            }
            state.anim_overall.update();
            let overall = *state.anim_overall;

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
    match check.status {
        CheckStatus::Pending => 0,
        CheckStatus::Running => check.progress.min(100),
        CheckStatus::Passed | CheckStatus::Failed | CheckStatus::Skipped | CheckStatus::Cached => {
            100
        }
    }
}

fn status_icon(check: &CheckProgress) -> &'static str {
    match check.status {
        CheckStatus::Passed => "◆",
        CheckStatus::Failed => "✖",
        CheckStatus::Running => SpinnerPreset::Anvil.frame(running_frame_index(check.start_time)),
        CheckStatus::Pending | CheckStatus::Skipped => "○",
        CheckStatus::Cached => "⚡",
    }
}

fn running_frame_index(start_time: Option<Instant>) -> usize {
    let interval_ms = SpinnerPreset::Anvil.interval().as_millis().max(1);
    let elapsed_ms = start_time.map_or(0, |started| {
        Instant::now()
            .saturating_duration_since(started)
            .as_millis()
    });
    usize::try_from(elapsed_ms / interval_ms).unwrap_or(0)
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
    let mut used = 0;
    for ch in name.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > width {
            break;
        }
        output.push(ch);
        used += cw;
    }

    if used < width {
        output.push_str(&" ".repeat(width - used));
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
    use ratatui::widgets::StatefulWidget;

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

    #[test]
    fn overall_progress_animates_toward_target() {
        use crate::widgets::ANIM_DURATION_MS;

        let theme = crate::theme::EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        let mut state = ParallelProgressState {
            checks: vec![CheckProgress {
                id: "a".to_string(),
                name: "lint".to_string(),
                status: CheckStatus::Passed,
                progress: 100,
                start_time: None,
                end_time: None,
                duration_ms: Some(100),
                message: None,
            }],
            start_time: None,
            ..Default::default()
        };

        // First render primes the animation toward overall=100.
        ParallelProgress::new(&theme).render(area, &mut buf, &mut state);
        let first = *state.anim_overall;
        assert!(
            first <= 100,
            "animation should start within [0, 100], got {first}"
        );

        // Advance past the configured animation duration and re-render.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let advance = ANIM_DURATION_MS as usize + 1;
        animate_core::tick(advance);
        ParallelProgress::new(&theme).render(area, &mut buf, &mut state);

        assert_eq!(
            *state.anim_overall, 100,
            "expected anim_overall to converge to target after full duration"
        );
    }

    #[test]
    fn compact_running_check_renders_spinner_frame() {
        let theme = crate::theme::EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        // Pin start_time so `running_frame_index` lands on frame 3 (the final
        // build-up glyph "‡"), letting us assert against a single distinctive
        // character that doesn't collide with the ETA dashes.
        let interval_ms = SpinnerPreset::Anvil.interval();
        let started = Instant::now()
            .checked_sub(interval_ms * 3 + Duration::from_millis(10))
            .expect("Instant arithmetic underflow");
        let mut state = ParallelProgressState {
            checks: vec![CheckProgress {
                id: "a".to_string(),
                name: "forge".to_string(),
                status: CheckStatus::Running,
                progress: 42,
                start_time: Some(started),
                end_time: None,
                duration_ms: None,
                message: Some("Forging".to_string()),
            }],
            start_time: None,
            ..Default::default()
        };

        ParallelProgress::new(&theme)
            .compact(true)
            .render(area, &mut buf, &mut state);

        let row: String = (0..40).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(
            row.contains("‡"),
            "expected the final anvil spinner frame (‡) in row {row:?}"
        );
    }
}

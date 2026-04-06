use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::hooks::{HookManager, HooksPhase, HooksState};

const PAD: &str = "  ";

pub fn render(frame: &mut Frame, area: Rect, state: &HooksState, theme: &EddaCraftTheme) {
    match state.phase {
        HooksPhase::Overview => render_overview(frame, area, state, theme),
        HooksPhase::Confirm => render_confirm(frame, area, state, theme),
        HooksPhase::Done => render_done(frame, area, state, theme),
    }
}

fn render_overview(frame: &mut Frame, area: Rect, state: &HooksState, theme: &EddaCraftTheme) {
    let manager_notice_height = if state.hook_manager == HookManager::None {
        0u16
    } else {
        2u16
    };

    let chunks = Layout::vertical([
        Constraint::Length(manager_notice_height),
        Constraint::Min(4),
    ])
    .split(area);

    if state.hook_manager != HookManager::None
        && let Some(note) = state.hook_manager.adapter_note()
    {
        let notice = Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled(
                    format!("Detected: {}", state.hook_manager.label()),
                    Style::default()
                        .fg(theme.warning())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled(note, Style::default().fg(theme.muted())),
            ]),
        ]));
        frame.render_widget(notice, chunks[0]);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Select Hooks to Install ");
    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    let hook_lines: Vec<Line> = state
        .hooks
        .iter()
        .enumerate()
        .flat_map(|(i, hook)| {
            let selected = i == state.cursor;
            let enabled = state.selected_hooks.get(i).copied().unwrap_or(false);

            let indicator = if selected { ">> " } else { "   " };
            let toggle_icon = if enabled { "[x]" } else { "[ ]" };
            let toggle_colour = if enabled {
                theme.success()
            } else {
                theme.muted()
            };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            let hook_line = Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled(indicator, name_style),
                Span::styled(
                    format!("{toggle_icon} "),
                    Style::default().fg(toggle_colour),
                ),
                Span::styled(hook.name, name_style),
            ]);
            let desc_line = Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled("       ", Style::default()),
                Span::styled(hook.description, Style::default().fg(theme.muted())),
            ]);
            let cmd_line = Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled("       runs: ", Style::default().fg(theme.muted())),
                Span::styled(
                    hook.command,
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);

            vec![hook_line, desc_line, cmd_line, Line::raw("")]
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(hook_lines)), inner);
}

fn render_confirm(frame: &mut Frame, area: Rect, state: &HooksState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.warning()))
        .title(" Confirm Installation ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let names = state.selected_hook_names();

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "The following hooks will be installed:",
            Style::default().fg(theme.fg()),
        )),
        Line::raw(""),
    ];

    if names.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no hooks selected — press esc to go back or enter to skip)",
            Style::default().fg(theme.muted()),
        )));
    } else {
        for name in &names {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent())),
                Span::styled(*name, Style::default().fg(theme.fg())),
            ]));
        }
    }

    if let Some(note) = state.hook_manager.adapter_note() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Note: ", Style::default().fg(theme.warning())),
            Span::styled(note, Style::default().fg(theme.muted())),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Press enter to install, esc to go back.",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_done(frame: &mut Frame, area: Rect, state: &HooksState, theme: &EddaCraftTheme) {
    let (border_colour, title, body_lines): (_, _, Vec<Line>) =
        if let Some(err) = &state.install_error {
            (
                theme.error(),
                " Installation Failed ",
                vec![
                    Line::from(Span::styled(
                        "Hook installation encountered an error:",
                        Style::default().fg(theme.fg()),
                    )),
                    Line::raw(""),
                    Line::from(vec![
                        Span::styled("  Error: ", Style::default().fg(theme.error())),
                        Span::styled(err.as_str(), Style::default().fg(theme.fg())),
                    ]),
                    Line::raw(""),
                    Line::from(Span::styled(
                        "Press enter or esc to continue.",
                        Style::default().fg(theme.muted()),
                    )),
                ],
            )
        } else if state.installed {
            let names = state.selected_hook_names();
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    "Git hooks installed successfully.",
                    Style::default()
                        .fg(theme.success())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
            ];
            for name in &names {
                lines.push(Line::from(vec![
                    Span::styled("  ✓ ", Style::default().fg(theme.success())),
                    Span::styled(*name, Style::default().fg(theme.fg())),
                ]));
            }
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Hooks will run on your next commit.",
                Style::default().fg(theme.muted()),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Press enter or esc to continue.",
                Style::default().fg(theme.muted()),
            )));
            (theme.success(), " Hooks Installed ", lines)
        } else {
            // Skipped (no hooks selected or user declined)
            (
                theme.muted(),
                " Skipped ",
                vec![
                    Line::from(Span::styled(
                        "Git hooks were not installed.",
                        Style::default().fg(theme.muted()),
                    )),
                    Line::raw(""),
                    Line::from(Span::styled(
                        "You can run `anvil hooks install` at any time to set this up later.",
                        Style::default().fg(theme.muted()),
                    )),
                    Line::raw(""),
                    Line::from(Span::styled(
                        "Press enter or esc to continue.",
                        Style::default().fg(theme.muted()),
                    )),
                ],
            )
        };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_colour))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(Text::from(body_lines)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn make_state() -> HooksState {
        let dir = std::env::temp_dir().join(format!(
            "anvil_hooks_render_test_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        HooksState::new(&dir)
    }

    #[test]
    fn renders_overview_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_confirm_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        state.phase = HooksPhase::Confirm;
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_done_installed_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        state.mark_installed();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_done_failed_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        state.mark_failed("permission denied".to_string());
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_done_skipped_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        state.phase = HooksPhase::Done;
        // installed=false, install_error=None → skipped path
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_overview_with_husky_detected() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        state.hook_manager = HookManager::Husky;
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_confirm_no_hooks_selected() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = make_state();
        for v in &mut state.selected_hooks {
            *v = false;
        }
        state.phase = HooksPhase::Confirm;
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}

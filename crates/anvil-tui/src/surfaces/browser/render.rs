use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{BrowserState, BrowserView};

pub fn render(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &EddaCraftTheme) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Breadcrumb
        Constraint::Min(6),    // Content area
    ])
    .split(area);

    // Breadcrumb
    let breadcrumb = match state.view {
        BrowserView::Categories => "Categories".to_string(),
        BrowserView::Templates => {
            let cat = state
                .categories
                .get(state.cat_selected)
                .map_or("?", |c| c.name.as_str());
            format!("Categories > {cat}")
        }
        BrowserView::Detail => {
            let cat = state
                .categories
                .get(state.cat_selected)
                .map_or("?", |c| c.name.as_str());
            let tmpl = state.selected_template().map_or("?", |t| t.name.as_str());
            format!("Categories > {cat} > {tmpl}")
        }
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Templates  ",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(breadcrumb, Style::default().fg(theme.muted())),
    ]));
    frame.render_widget(title, chunks[0]);

    // Content
    match state.view {
        BrowserView::Categories => render_categories(frame, chunks[1], state, theme),
        BrowserView::Templates => render_templates(frame, chunks[1], state, theme),
        BrowserView::Detail => render_detail(frame, chunks[1], state, theme),
    }
}

fn render_categories(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Categories ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let selected = i == state.cat_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(&cat.name, name_style),
                Span::styled(
                    format!("  ({} templates)", cat.template_count),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(
                    format!("  {}", cat.description),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_templates(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Templates ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let filtered = state.filtered_templates();

    if filtered.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No templates match the current filter",
            Style::default().fg(theme.muted()),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    let items: Vec<Line> = filtered
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let selected = i == state.tmpl_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let tags = t.tags.join(", ");

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(&t.name, name_style),
                Span::styled(
                    format!("  {}", t.description),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(format!("  [{tags}]"), Style::default().fg(theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_detail(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &EddaCraftTheme) {
    let Some(template) = state.selected_template() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.muted()))
            .title(" Detail ");
        frame.render_widget(block, area);
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(format!(" {} ", template.name));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name:        ", Style::default().fg(theme.muted())),
            Span::styled(
                &template.name,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(theme.muted())),
            Span::styled(&template.description, Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Tags:        ", Style::default().fg(theme.muted())),
            Span::styled(template.tags.join(", "), Style::default().fg(theme.fg())),
        ]),
        Line::default(),
    ];

    if template.variables.is_empty() {
        lines.push(Line::from(Span::styled(
            "No variables required",
            Style::default().fg(theme.muted()),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Variables:",
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        )));
        for (i, var) in template.variables.iter().enumerate() {
            let selected = i == state.var_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };
            let required_marker = if var.required { " *" } else { "" };
            let default_text = var
                .default_value
                .as_ref()
                .map_or(String::new(), |d| format!("  (default: {d})"));

            lines.push(Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(&var.name, name_style),
                Span::styled(required_marker, Style::default().fg(theme.error())),
                Span::styled(
                    format!("  {}", var.description),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(default_text, Style::default().fg(theme.muted())),
            ]));
        }
    }

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Press enter to use this template",
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_state() -> BrowserState {
        use super::super::{TemplateCategory, TemplateEntry, TemplateVariable};

        let categories = vec![
            TemplateCategory {
                name: "Starter".to_string(),
                description: "Basic templates".to_string(),
                template_count: 1,
            },
            TemplateCategory {
                name: "Advanced".to_string(),
                description: "Full-featured".to_string(),
                template_count: 1,
            },
        ];
        let templates = vec![
            TemplateEntry {
                id: "basic-ts".to_string(),
                name: "Basic TypeScript".to_string(),
                description: "Minimal TS project".to_string(),
                category: "Starter".to_string(),
                tags: vec!["typescript".to_string()],
                variables: vec![TemplateVariable {
                    name: "project_name".to_string(),
                    description: "Name of the project".to_string(),
                    default_value: Some("my-project".to_string()),
                    required: true,
                }],
            },
            TemplateEntry {
                id: "monorepo".to_string(),
                name: "Monorepo".to_string(),
                description: "Multi-package workspace".to_string(),
                category: "Advanced".to_string(),
                tags: vec!["monorepo".to_string()],
                variables: vec![],
            },
        ];

        BrowserState::new(categories, templates)
    }

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_categories_view() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(frame, frame.area(), Surface::surface_name(&state), Surface::help_text(&state), &theme);
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_templates_view() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.view = BrowserView::Templates;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(frame, frame.area(), Surface::surface_name(&state), Surface::help_text(&state), &theme);
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_detail_view() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.view = BrowserView::Detail;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(frame, frame.area(), Surface::surface_name(&state), Surface::help_text(&state), &theme);
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::json_render::{self, Catalog};
    use eddacraft_tui::prelude::{EddaCraftTheme, PretextState, PretextWidget};
    use eddacraft_tui::pretext::{ExclusionZone, PreparedText, layout};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use serde_json::json;

    const SPEC: &str = r#"{
      "title": "Public crate smoke",
      "version": "1.0",
      "root": "page",
      "elements": {
        "page": {
          "type": "Stack",
          "props": { "gap": "sm" },
          "children": ["heading", "summary", "metric"]
        },
        "heading": {
          "type": "Heading",
          "props": { "children": "eddacraft-tui", "level": 2 },
          "children": []
        },
        "summary": {
          "type": "Text",
          "props": { "children": "json-render from crates.io rendered this" },
          "children": []
        },
        "metric": {
          "type": "MetricCard",
          "props": { "label": "pretext", "value": "ok", "trend": "up" },
          "children": []
        }
      }
    }"#;

    #[test]
    fn json_render_public_crate_parses_validates_and_renders() {
        let spec = json_render::parse(SPEC).expect("published crate parses json-render spec");
        json_render::validate(&spec, &Catalog::base()).expect("base catalogue accepts smoke spec");

        let pretty = json_render::to_json_pretty(&spec).expect("serialise spec");
        let reparsed = json_render::parse(&pretty).expect("reparse pretty spec");
        assert_eq!(spec, reparsed, "json-render semantic round trip changed");

        let registry = json_render::base_registry();
        let mut terminal = Terminal::new(TestBackend::new(96, 24)).expect("test backend");
        terminal
            .draw(|frame| json_render::render_spec(&spec, &registry, frame, frame.area()))
            .expect("render public json-render spec");

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("eddacraft-tui"), "heading should render");
        assert!(rendered.contains("json-render from crates.io rendered this"));
        assert!(!rendered.contains("not available in terminal"));
        assert!(!rendered.contains("[missing:"));
    }

    #[test]
    fn json_render_public_crate_sanitises_control_sequences() {
        let mut spec = json_render::parse(SPEC).expect("parse smoke spec");
        let hostile = "safe\u{1b}]52;c;cHk\u{07}text\u{1b}]0;pwned\u{07}";
        spec.elements
            .get_mut("summary")
            .expect("summary element")
            .props
            .insert("children".to_owned(), json!(hostile));

        let registry = json_render::base_registry();
        let mut terminal = Terminal::new(TestBackend::new(96, 8)).expect("test backend");
        terminal
            .draw(|frame| json_render::render_spec(&spec, &registry, frame, frame.area()))
            .expect("render hostile spec");

        let offenders: Vec<u32> = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .flat_map(|cell| cell.symbol().chars())
            .filter(|ch| char::is_control(*ch))
            .map(|ch| ch as u32)
            .collect();
        assert!(
            offenders.is_empty(),
            "control bytes reached rendered buffer: {offenders:?}"
        );
    }

    #[test]
    fn pretext_public_crate_prepares_lays_out_and_wraps_around_exclusion() {
        let mut prepared = PreparedText::new("public pretext layout keeps terminal text stable");
        prepared.append(" while wrapping around reserved cells");

        let exclusion = ExclusionZone::rect(0, 1, 8, 1);
        let laid_out = layout(&prepared, 24, &[exclusion]);

        assert!(laid_out.total_height >= 2, "text should wrap across rows");
        assert!(
            laid_out
                .lines
                .iter()
                .flat_map(|line| &line.words)
                .all(|word| word.y != 1 || word.x >= 8),
            "words on excluded row must start after the reserved band"
        );
        let words: Vec<&str> = laid_out
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .map(|word| word.text.as_str())
            .collect();
        assert!(words.contains(&"public"));
        assert!(words.contains(&"pretext"));
    }

    #[test]
    fn pretext_public_widget_renders_with_default_surface_types() {
        let theme = EddaCraftTheme;
        let mut state = PretextState::new("pretext widget from the published crate renders");
        state.append(" with cached layout");
        let widget = PretextWidget::themed(&theme);

        let mut terminal = Terminal::new(TestBackend::new(48, 6)).expect("test backend");
        terminal
            .draw(|frame| frame.render_stateful_widget(widget, Rect::new(0, 0, 48, 6), &mut state))
            .expect("render pretext widget");

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(rendered.contains("pretext widget"));
        assert!(rendered.contains("published crate"));
    }
}

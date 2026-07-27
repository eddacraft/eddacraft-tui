use anvil_dashboard_server::{DataState, Workspace, load_protection_history};

fn workspace_with_history(contents: Option<&str>) -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir().expect("workspace");
    if let Some(contents) = contents {
        std::fs::create_dir(root.path().join(".anvil")).unwrap();
        std::fs::write(root.path().join(".anvil/gate-history.ndjson"), contents).unwrap();
    }
    let workspace = Workspace::new(root.path()).unwrap();
    (root, workspace)
}

#[test]
fn missing_empty_and_fully_corrupt_history_are_unavailable() {
    for contents in [None, Some(""), Some("not-json\n{broken\n")] {
        let (_root, workspace) = workspace_with_history(contents);
        let history = load_protection_history(&workspace);
        assert_eq!(history.data_state, DataState::Unavailable);
        assert!(history.points.is_empty());
        assert!(history.actual_range.is_none());
        assert!(
            history
                .gaps
                .iter()
                .any(|gap| gap.component == "gate-history")
        );
        assert!(
            history
                .gaps
                .iter()
                .any(|gap| gap.component == "drift-history")
        );
        assert!(
            history
                .gaps
                .iter()
                .any(|gap| gap.component == "suppression-history")
        );
    }
}

#[test]
fn valid_points_are_ordered_and_report_the_actual_range() {
    let (_root, workspace) = workspace_with_history(Some(concat!(
        "{\"recorded_at\":\"2026-07-03T12:00:00Z\",\"score\":80,\"status\":\"warn\",\"status_label\":\"warn\",\"warning_count\":2}\n",
        "{\"recorded_at\":\"2026-07-01T12:00:00Z\",\"score\":100,\"status\":\"pass\",\"status_label\":\"pass\",\"warning_count\":0}\n"
    )));
    let history = load_protection_history(&workspace);

    assert_eq!(history.data_state, DataState::Complete);
    assert_eq!(history.points[0].recorded_at, "2026-07-01T12:00:00Z");
    assert_eq!(history.points[1].recorded_at, "2026-07-03T12:00:00Z");
    let range = history.actual_range.expect("actual range");
    assert_eq!(range.first_recorded_at, "2026-07-01T12:00:00Z");
    assert_eq!(range.last_recorded_at, "2026-07-03T12:00:00Z");
}

#[test]
fn mixed_valid_and_corrupt_lines_are_partial_without_hiding_the_gap() {
    let (_root, workspace) = workspace_with_history(Some(concat!(
        "{\"recorded_at\":\"2026-07-01T12:00:00Z\",\"score\":100,\"status\":\"pass\",\"status_label\":\"pass\",\"warning_count\":0}\n",
        "not-json\n"
    )));
    let history = load_protection_history(&workspace);

    assert_eq!(history.data_state, DataState::Partial);
    assert_eq!(history.points.len(), 1);
    assert!(history.source_message.contains("1 invalid"));
    assert!(
        history
            .gaps
            .iter()
            .any(|gap| gap.component == "gate-history")
    );
}

#[test]
fn impossible_calendar_dates_are_invalid_history_points() {
    let (_root, workspace) = workspace_with_history(Some(concat!(
        "{\"recorded_at\":\"2026-02-31T12:00:00Z\",\"score\":100,\"status\":\"pass\",\"status_label\":\"pass\",\"warning_count\":0}\n",
        "{\"recorded_at\":\"2026-03-01T12:00:00Z\",\"score\":90,\"status\":\"warn\",\"status_label\":\"warn\",\"warning_count\":1}\n"
    )));

    let history = load_protection_history(&workspace);

    assert_eq!(history.data_state, DataState::Partial);
    assert_eq!(history.points.len(), 1);
    assert_eq!(history.points[0].recorded_at, "2026-03-01T12:00:00Z");
    assert!(history.source_message.contains("1 invalid"));
}

#[test]
fn keeps_only_newest_500_valid_points_and_reports_the_cap_gap() {
    let contents = (0..501)
        .map(|index| {
            format!(
                "{{\"recorded_at\":\"2026-07-{:02}T12:{:02}:00Z\",\"score\":100,\"status\":\"pass\",\"status_label\":\"point-{index}\",\"warning_count\":0}}",
                1 + index / 60,
                index % 60
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let (_root, workspace) = workspace_with_history(Some(&contents));

    let history = load_protection_history(&workspace);

    assert_eq!(history.data_state, DataState::Partial);
    assert_eq!(history.points.len(), 500);
    assert_eq!(history.points.first().unwrap().status_label, "point-1");
    assert_eq!(history.points.last().unwrap().status_label, "point-500");
    assert!(
        history
            .gaps
            .iter()
            .any(|gap| gap.component == "gate-history-cap")
    );
}

#[test]
fn rejects_out_of_range_scores_without_hiding_valid_points() {
    let (_root, workspace) = workspace_with_history(Some(concat!(
        "{\"recorded_at\":\"2026-07-01T12:00:00Z\",\"score\":-1,\"status\":\"fail\",\"status_label\":\"negative\",\"warning_count\":1}\n",
        "{\"recorded_at\":\"2026-07-02T12:00:00Z\",\"score\":101,\"status\":\"pass\",\"status_label\":\"too-high\",\"warning_count\":0}\n",
        "{\"recorded_at\":\"2026-07-03T12:00:00Z\",\"score\":80,\"status\":\"warn\",\"status_label\":\"valid\",\"warning_count\":1}\n"
    )));

    let history = load_protection_history(&workspace);

    assert_eq!(history.data_state, DataState::Partial);
    assert_eq!(history.points.len(), 1);
    assert!((history.points[0].score - 80.0).abs() < f64::EPSILON);
    assert!(history.source_message.contains("2 invalid"));
}

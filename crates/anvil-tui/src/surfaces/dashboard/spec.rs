//! Spec-rendering dashboard surface (TUIDASH-009).
//!
//! Renders a saved json-render dashboard spec (`.anvil/dashboards/<name>.json`)
//! through the engine, alongside the fixed native dashboards. The CLI parses the
//! spec and hands it here; the surface owns binding (`$data` → `.anvil/` values)
//! and re-binding on refresh.
//!
//! Keys: `q`/`esc` quit, `r` refresh (re-read `.anvil/` data and re-bind). The
//! spec itself is not re-read on refresh — only its data — so a running
//! dashboard reflects fresh gate/drift state without restarting.

use std::fs;
use std::path::{Path, PathBuf};

use eddacraft_tui::json_render::{RenderSpec, TuiRegistry, bind, parse, render_spec, sanitize};
use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::dashboard_catalog::anvil_registry;
use crate::dashboard_context::load_context;
use crate::surface::Surface;

/// Surface state for a single rendered dashboard spec.
pub struct SpecDashboardState {
    title: String,
    /// The authored spec, kept unbound so refresh can re-bind it against fresh
    /// data without re-reading the file.
    spec: RenderSpec,
    /// Workspace root whose `.anvil/` provides the data context.
    root: PathBuf,
    /// The component registry (generic base + Anvil domain) specs render against.
    registry: TuiRegistry,
    /// The spec with `$data` references resolved — what is rendered. Rebuilt on
    /// refresh.
    bound: RenderSpec,
    should_quit: bool,
}

impl SpecDashboardState {
    /// Build a surface for `spec`, binding it against `<root>/.anvil/` data.
    #[must_use]
    pub fn new(spec: RenderSpec, root: PathBuf) -> Self {
        let registry = anvil_registry();
        let bound = bind(&spec, &load_context(&root));
        Self {
            // The title is shown in the surface chrome — sanitise control bytes.
            title: sanitize(&spec.title),
            spec,
            root,
            registry,
            bound,
            should_quit: false,
        }
    }

    /// Re-read `.anvil/` data and re-bind the spec (the `r` refresh action).
    fn refresh(&mut self) {
        self.bound = bind(&self.spec, &load_context(&self.root));
    }

    /// The dashboard title (from the spec).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether the spec uses `visible` conditions on any element. The renderer
    /// does not yet evaluate them (every element renders), so the CLI warns the
    /// operator that conditional sections will always show.
    #[must_use]
    pub fn has_unevaluated_visibility(&self) -> bool {
        self.spec.elements.values().any(|e| e.visible.is_some())
    }
}

/// Maximum size of a dashboard spec file. A spec is a small JSON document; a
/// multi-megabyte file is pathological (or hostile), and reading/parsing it
/// whole would stall the picker, so oversized files are skipped (discovery) or
/// rejected (load) before `read_to_string` buffers them.
const MAX_SPEC_BYTES: u64 = 2 * 1024 * 1024;

/// A saved dashboard spec discovered under `.anvil/dashboards/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedDashboard {
    /// Machine name (the file stem) used on the command line.
    pub name: String,
    /// Human title (the spec's `title`) shown in the picker.
    pub title: String,
    /// Path to the spec file.
    pub path: PathBuf,
}

/// Why loading a saved dashboard spec failed.
#[derive(Debug, thiserror::Error)]
pub enum SpecLoadError {
    /// The spec file could not be read.
    #[error("could not read dashboard spec {path}: {source}")]
    Read {
        /// The path that failed to read.
        path: String,
        /// The underlying IO error.
        source: std::io::Error,
    },
    /// The spec file was not valid json-render JSON.
    #[error("could not parse dashboard spec {path}: {source}")]
    Parse {
        /// The path that failed to parse.
        path: String,
        /// The underlying JSON error.
        source: serde_json::Error,
    },
    /// The spec file exceeds [`MAX_SPEC_BYTES`].
    #[error("dashboard spec {path} is too large ({size} bytes, max {MAX_SPEC_BYTES})")]
    TooLarge {
        /// The oversized path.
        path: String,
        /// Its size in bytes.
        size: u64,
    },
    /// The path is not a regular file (e.g. a symlink, directory, or device).
    #[error("dashboard spec {path} is not a regular file")]
    NotRegularFile {
        /// The offending path.
        path: String,
    },
}

/// List the saved dashboard specs under `<root>/.anvil/dashboards/`, sorted by
/// name. Files that are not `*.json` or that fail to parse are skipped (a
/// malformed saved dashboard simply does not list), so discovery never errors.
#[must_use]
pub fn discover(root: &Path) -> Vec<SavedDashboard> {
    let dir = root.join(".anvil").join("dashboards");
    // Reject a symlinked container directory before iterating — `read_dir`
    // would otherwise follow it out of the workspace.
    if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // Require a regular file (no-follow): a symlink could point at a device
        // (`/dev/zero`) or a secret outside the workspace. Skip it before opening.
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        // Bounded read from a single handle (caps size; cannot hang on a
        // device/FIFO swapped in after the check). `Ok(None)` = over cap.
        let Ok(Some(text)) = crate::fileio::read_capped(&path, MAX_SPEC_BYTES) else {
            continue;
        };
        if let Ok(spec) = parse(&text) {
            out.push(SavedDashboard {
                name: name.to_owned(),
                // Shown in the picker list — sanitise control bytes.
                title: sanitize(&spec.title),
                path: path.clone(),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Read a saved spec file's verbatim text, rejecting non-regular files and
/// enforcing the size cap with a bounded read.
///
/// `symlink_metadata` (no-follow) rejects a symlink/device/dir up front; the
/// read is then hard-bounded so a target swapped in afterwards cannot hang or
/// exhaust memory. Used by [`load`] and by the CLI's `--json` verbatim path.
///
/// # Errors
/// [`SpecLoadError`] if the path is not a regular file, exceeds [`MAX_SPEC_BYTES`],
/// or cannot be read.
pub fn read_raw(path: &Path) -> Result<String, SpecLoadError> {
    let display = || path.display().to_string();
    let meta = fs::symlink_metadata(path).map_err(|source| SpecLoadError::Read {
        path: display(),
        source,
    })?;
    if !meta.file_type().is_file() {
        return Err(SpecLoadError::NotRegularFile { path: display() });
    }
    match crate::fileio::read_capped(path, MAX_SPEC_BYTES) {
        Ok(Some(text)) => Ok(text),
        Ok(None) => Err(SpecLoadError::TooLarge {
            path: display(),
            size: meta.len(),
        }),
        Err(source) => Err(SpecLoadError::Read {
            path: display(),
            source,
        }),
    }
}

/// Parse a spec from an embedded string (no file read), returning a surface
/// bound against `<root>/.anvil/`. Serves built-in dashboards that ship inside
/// the binary, e.g. the gate-summary fallback for projects whose
/// `.anvil/dashboards/` predates init seeding (UJ-009).
///
/// # Errors
/// [`SpecLoadError::Parse`] if the text is not valid json-render JSON.
pub fn load_str(text: &str, root: PathBuf) -> Result<SpecDashboardState, SpecLoadError> {
    let spec = parse(text).map_err(|source| SpecLoadError::Parse {
        path: "<embedded>".to_string(),
        source,
    })?;
    Ok(SpecDashboardState::new(spec, root))
}

/// Read and parse the saved spec at `path`, returning a surface bound against
/// `<root>/.anvil/`.
///
/// # Errors
/// [`SpecLoadError`] if the file cannot be read, is not a regular file, exceeds
/// the size cap, or is not valid json-render JSON.
pub fn load(path: &Path, root: PathBuf) -> Result<SpecDashboardState, SpecLoadError> {
    let text = read_raw(path)?;
    let spec = parse(&text).map_err(|source| SpecLoadError::Parse {
        path: path.display().to_string(),
        source,
    })?;
    Ok(SpecDashboardState::new(spec, root))
}

impl Surface for SpecDashboardState {
    fn surface_name(&self) -> &str {
        &self.title
    }

    fn help_text(&self) -> &'static str {
        "r refresh  esc/q quit"
    }

    fn handle_key(&mut self, action: Action) {
        match action {
            Action::Quit | Action::Back => self.should_quit = true,
            Action::Character('r') => self.refresh(),
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(&self, frame: &mut Frame, area: Rect, _theme: &EddaCraftTheme) {
        render_spec(&self.bound, &self.registry, frame, area);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eddacraft_tui::json_render::parse;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    const SPEC: &str = r#"{
        "title": "Gate", "version": "1.0", "root": "page",
        "elements": {
            "page": { "type": "Stack", "props": {}, "children": ["m"] },
            "m": { "type": "MetricCard",
                   "props": { "label": "Pass Rate", "value": { "$data": "gates.passRate" } },
                   "children": [] }
        }
    }"#;

    fn render_text(state: &SpecDashboardState, w: u16, h: u16) -> String {
        let theme = EddaCraftTheme;
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("backend");
        terminal
            .draw(|frame| state.render(frame, frame.area(), &theme))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut state =
            SpecDashboardState::new(parse(SPEC).expect("parse"), tmp.path().to_path_buf());
        assert!(!state.should_quit());
        state.handle_key(Action::Quit);
        assert!(state.should_quit());
    }

    #[test]
    fn title_comes_from_the_spec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SpecDashboardState::new(parse(SPEC).expect("parse"), tmp.path().to_path_buf());
        assert_eq!(state.surface_name(), "Gate");
    }

    #[test]
    fn discover_lists_parseable_saved_specs_sorted_and_skips_bad_ones() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join(".anvil").join("dashboards");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("zeta.json"), SPEC).expect("write");
        fs::write(
            dir.join("alpha.json"),
            r#"{ "title": "Alpha", "version": "1.0", "root": "p",
                 "elements": { "p": { "type": "Text", "props": {}, "children": [] } } }"#,
        )
        .expect("write");
        fs::write(dir.join("broken.json"), "{ not json").expect("write");
        fs::write(dir.join("notes.txt"), "ignore").expect("write");

        let found = discover(tmp.path());
        let names: Vec<&str> = found.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["alpha", "zeta"], "sorted, parseable only");
        assert_eq!(found[0].title, "Alpha");
        assert_eq!(found[1].title, "Gate");
    }

    #[test]
    fn discover_on_missing_dir_is_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(discover(tmp.path()).is_empty());
    }

    #[test]
    fn detects_unevaluated_visibility_conditions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let with_vis = parse(
            r#"{ "title": "v", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Text", "props": {}, "children": [],
                     "visible": { "field": "showAdvanced" } } } }"#,
        )
        .expect("parse");
        let s = SpecDashboardState::new(with_vis, tmp.path().to_path_buf());
        assert!(s.has_unevaluated_visibility(), "non-null visible detected");

        let without =
            SpecDashboardState::new(parse(SPEC).expect("parse"), tmp.path().to_path_buf());
        assert!(
            !without.has_unevaluated_visibility(),
            "no visible conditions -> false"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discover_skips_a_symlinked_dashboards_directory() {
        // A symlinked container directory must not be traversed out of the tree.
        let tmp = tempfile::tempdir().expect("tempdir");
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir");
        let outside = tmp.path().join("elsewhere");
        fs::create_dir_all(&outside).expect("mkdir outside");
        fs::write(outside.join("leak.json"), SPEC).expect("write");
        std::os::unix::fs::symlink(&outside, anvil.join("dashboards")).expect("symlink dir");

        assert!(
            discover(tmp.path()).is_empty(),
            "a symlinked dashboards/ directory yields no specs"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discover_skips_symlinked_specs_and_load_rejects_them() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join(".anvil").join("dashboards");
        fs::create_dir_all(&dir).expect("mkdir");
        // A real spec lists; a symlink to a valid spec does not.
        fs::write(dir.join("real.json"), SPEC).expect("write");
        let target = tmp.path().join("outside.json");
        fs::write(&target, SPEC).expect("write target");
        std::os::unix::fs::symlink(&target, dir.join("link.json")).expect("symlink");

        let found = discover(tmp.path());
        let names: Vec<&str> = found.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["real"], "symlinked spec is not discovered");

        // load() on a symlink path is rejected, not followed.
        assert!(matches!(
            load(&dir.join("link.json"), tmp.path().to_path_buf()),
            Err(SpecLoadError::NotRegularFile { .. })
        ));
    }

    #[test]
    fn load_reads_parses_and_reports_errors() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("d.json");
        fs::write(&path, SPEC).expect("write");
        let state = load(&path, tmp.path().to_path_buf()).expect("loads");
        assert_eq!(state.title(), "Gate");

        fs::write(&path, "{ bad").expect("write");
        assert!(matches!(
            load(&path, tmp.path().to_path_buf()),
            Err(SpecLoadError::Parse { .. })
        ));

        let missing = tmp.path().join("nope.json");
        assert!(matches!(
            load(&missing, tmp.path().to_path_buf()),
            Err(SpecLoadError::Read { .. })
        ));
    }

    #[test]
    fn unbound_data_renders_an_em_dash_then_refresh_picks_up_values() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // No .anvil/ yet: the $data path misses, MetricCard shows an em dash.
        let mut state =
            SpecDashboardState::new(parse(SPEC).expect("parse"), tmp.path().to_path_buf());
        let before = render_text(&state, 30, 4);
        assert!(
            before.contains('—'),
            "missing data shows em dash: {before:?}"
        );
        assert!(!before.contains("94%"));

        // Write the data and refresh: the bound value updates.
        let anvil = tmp.path().join(".anvil");
        fs::create_dir_all(&anvil).expect("mkdir");
        fs::write(anvil.join("gates.json"), r#"{ "passRate": "94%" }"#).expect("write");
        state.handle_key(Action::Character('r'));
        let after = render_text(&state, 30, 4);
        assert!(
            after.contains("94%"),
            "refresh picks up new data: {after:?}"
        );
    }
}

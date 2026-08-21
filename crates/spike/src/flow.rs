//! Spike: anvil boundary graph rendered with rataflow, fed from the anvil
//! daemon's warm graph-cache snapshot.
//!
//! Validates rataflow (node-based flow graphs for ratatui) as a candidate for
//! an interactive boundary/impact view in the anvil TUI, and proves the warm
//! graph-cache snapshot can serve as its data source without daemon changes.
//!
//! Usage:
//! ```text
//! cargo run -p anvil-spike --bin spike-flow                 # current repo
//! cargo run -p anvil-spike --bin spike-flow -- --focus anvil-graph-cache
//! cargo run -p anvil-spike --bin spike-flow -- edges.json   # explicit JSON
//! ```
//!
//! With no arguments the spike locates the daemon's warm graph-cache snapshot
//! for the current directory (~/.local/state/anvil/graph-cache/*.snap, ADR-069)
//! and derives two graphs from its file-level import data: a crate-level graph
//! of *used* imports (not declared Cargo deps), and per-crate internal module
//! graphs. A JSON file of `{"edges": [[from, to], …]}` can be given instead.
//!
//! Interactive:
//! ```text
//! click / ↑↓←→ select a node (arrows navigate spatially, Tab cycles)
//! z            zoom-to-read: centre the selection and snap to 1:1
//! + / - / 0    zoom in / out / reset to 1:1   (scroll wheel also zooms)
//! hjkl         pan
//! Enter        drill into the selected crate's neighbourhood
//! i            drill into the selected crate's internal module graph
//! Esc / b      back up one drill level
//! drag on ●    create a new (proposed) edge between nodes
//! Delete       remove selected nodes (and their edges) from the model
//! f            fit view    q  quit
//! ```
//!
//! `--snapshot [WxH]` renders one frame to a headless buffer and prints it;
//! `--zoom-read <node>` does the same after centring that node at 1:1.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::io;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
};
use rataflow::{Background, Flow, FlowEvent, StepEdge, Sugiyama};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde::Deserialize;

#[derive(Deserialize)]
struct GraphFile {
    edges: Vec<(String, String)>,
}

struct Args {
    focus: Option<String>,
    internals: Option<String>,
    snapshot: Option<(u16, u16)>,
    fit: bool,
    anvil_snap: Option<String>,
    zoom_read: Option<String>,
    path: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        focus: None,
        internals: None,
        snapshot: None,
        fit: true,
        anvil_snap: None,
        zoom_read: None,
        path: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--focus" => args.focus = it.next(),
            "--internals" => args.internals = it.next(),
            "--no-fit" => args.fit = false,
            "--anvil-snap" => args.anvil_snap = it.next(),
            "--zoom-read" => args.zoom_read = it.next(),
            "--snapshot" => {
                let dims = it
                    .next()
                    .and_then(|s| {
                        let (w, h) = s.split_once('x')?;
                        Some((w.parse().ok()?, h.parse().ok()?))
                    })
                    .unwrap_or((160, 45));
                args.snapshot = Some(dims);
            }
            other => args.path = Some(other.to_string()),
        }
    }
    args
}

// ---------------------------------------------------------------------------
// Data sources
// ---------------------------------------------------------------------------

fn load_json_edges(path: &str) -> io::Result<Vec<(String, String)>> {
    let raw = std::fs::read_to_string(path)?;
    let graph: GraphFile = serde_json::from_str(&raw).map_err(io::Error::other)?;
    Ok(graph.edges)
}

/// Raw file-level import data from the anvil snapshot:
/// `(source file path, import specifier)` pairs plus the tracked-file set.
struct RawGraph {
    edges: Vec<(String, String)>,
    files: BTreeSet<String>,
}

/// Locate and decode the warm graph-cache snapshot for `root`.
fn load_anvil_snapshot(root: &str) -> io::Result<RawGraph> {
    use anvil_graph_cache::snapshot::SnapshotPayload;

    let root = std::fs::canonicalize(root)?;
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state"))
        })
        .ok_or_else(|| io::Error::other("neither XDG_STATE_HOME nor HOME is set"))?
        .join("anvil/graph-cache");

    let mut snap_path = None;
    for entry in std::fs::read_dir(&state)? {
        let entry = entry?;
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "root")
            && std::fs::read_to_string(&p)?.trim() == root.to_string_lossy()
        {
            snap_path = Some(p.with_extension("snap"));
            break;
        }
    }
    let snap_path = snap_path.ok_or_else(|| {
        io::Error::other(format!("no graph-cache snapshot for {}", root.display()))
    })?;

    let bytes = std::fs::read(&snap_path)?;
    let payload = SnapshotPayload::from_bytes(&bytes)
        .map_err(|e| io::Error::other(format!("snapshot decode: {e:?}")))?;
    let files: BTreeSet<String> = payload
        .tracked_files()
        .iter()
        .map(ToString::to_string)
        .collect();
    let (_symbols, deps) = payload
        .into_graphs()
        .map_err(|e| io::Error::other(format!("snapshot graphs: {e:?}")))?;

    let mut edges = Vec::new();
    for file in &files {
        for target in deps.dependencies_of(file) {
            edges.push((file.clone(), target.to_string()));
        }
    }
    Ok(RawGraph { edges, files })
}

/// The crate a workspace-relative file belongs to, if any.
fn crate_of(path: &str) -> Option<&str> {
    path.strip_prefix("crates/")?.split('/').next()
}

/// Crate-level graph from raw import edges: for Rust files under `crates/`,
/// the first segment of a `use` path is matched against workspace lib names
/// (`anvil_checks` → `anvil-checks`). `std`/`crate`/`super`/external crates
/// drop out; what remains is the *used* cross-crate dependency graph.
fn crate_level(raw: &RawGraph) -> Vec<(String, String)> {
    let lib_to_crate: HashMap<String, String> = raw
        .files
        .iter()
        .filter_map(|f| crate_of(f))
        .map(|c| (c.replace('-', "_"), c.to_string()))
        // the eddacraft-anvil-* packages export anvil_* lib names
        .flat_map(|(lib, c)| {
            let stripped = lib.trim_start_matches("eddacraft_").to_string();
            [(lib, c.clone()), (stripped, c)]
        })
        .collect();

    let mut edges = BTreeSet::new();
    for (file, spec) in &raw.edges {
        let Some(from) = crate_of(file) else { continue };
        let Some(seg) = spec.split("::").next() else {
            continue;
        };
        if let Some(to) = lib_to_crate.get(seg)
            && to != from
        {
            edges.insert((from.to_string(), to.clone()));
        }
    }
    edges.into_iter().collect()
}

/// File-level internal graph of one crate: `crate::a::b` specifiers are
/// resolved to files by longest path-prefix match (`src/a/b.rs`,
/// `src/a/b/mod.rs`, or the deepest existing prefix).
fn internals_of(raw: &RawGraph, krate: &str) -> Vec<(String, String)> {
    let prefix = format!("crates/{krate}/");
    let label = |f: &str| f.strip_prefix(&prefix).unwrap_or(f).to_string();

    let resolve = |module_path: &[&str]| -> Option<String> {
        // try deepest → shallowest: src/a/b.rs, src/a/b/mod.rs, src/a.rs …
        for depth in (1..=module_path.len()).rev() {
            let joined = module_path[..depth].join("/");
            for cand in [
                format!("{prefix}src/{joined}.rs"),
                format!("{prefix}src/{joined}/mod.rs"),
            ] {
                if raw.files.contains(&cand) {
                    return Some(label(&cand));
                }
            }
        }
        None
    };

    let mut edges = BTreeSet::new();
    for (file, spec) in &raw.edges {
        if !file.starts_with(&prefix) {
            continue;
        }
        let segs: Vec<&str> = spec.split("::").collect();
        if segs.first() != Some(&"crate") || segs.len() < 2 {
            continue;
        }
        if let Some(to) = resolve(&segs[1..]) {
            let from = label(file);
            if from != to {
                edges.insert((from, to));
            }
        }
    }
    // fall back so a crate with no resolvable internal edges still shows a node
    if edges.is_empty()
        && let Some(f) = raw.files.iter().find(|f| f.starts_with(&prefix))
    {
        edges.insert((label(f), "(no internal crate:: edges)".into()));
    }
    edges.into_iter().collect()
}

// ---------------------------------------------------------------------------
// App model: drill-down stack over a mutable edge set
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum View {
    All,
    Focus(String),
    Internals(String),
}

impl View {
    fn name(&self) -> String {
        match self {
            View::All => "all".into(),
            View::Focus(n) => n.clone(),
            View::Internals(n) => format!("{n} internals"),
        }
    }
}

struct App {
    /// Crate-level edge set — source of truth for All/Focus views.
    model: Vec<(String, String)>,
    /// Raw file-level import data (snapshot mode only) for Internals views.
    raw: Option<RawGraph>,
    stack: Vec<View>,
    selected: Option<String>,
    /// Edges added interactively this session (proposed dependencies).
    added: Vec<(String, String)>,
    status: String,
}

impl App {
    fn current_edges(&self) -> Vec<(String, String)> {
        match self.stack.last().unwrap_or(&View::All) {
            View::All => self.model.clone(),
            View::Focus(focus) => self
                .model
                .iter()
                .filter(|(a, b)| a == focus || b == focus)
                .cloned()
                .collect(),
            View::Internals(krate) => match &self.raw {
                Some(raw) => internals_of(raw, krate),
                None => Vec::new(),
            },
        }
    }

    fn breadcrumb(&self) -> String {
        self.stack
            .iter()
            .map(View::name)
            .collect::<Vec<_>>()
            .join(" › ")
    }

    fn build_flow(&self) -> io::Result<Flow> {
        let edges = self.current_edges();
        if edges.is_empty() {
            return Err(io::Error::other("current view has no edges"));
        }
        let pairs: Vec<(&str, &str)> = edges
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        Flow::from_edges(&pairs, Sugiyama::vertical()).map_err(io::Error::other)
    }

    fn push_view(&mut self, view: View) -> bool {
        if self.stack.last() == Some(&view) {
            return false;
        }
        self.status = format!("→ {}", view.name());
        self.stack.push(view);
        true
    }

    fn back(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.status = "back".to_string();
            true
        } else {
            false
        }
    }

    fn add_edge(&mut self, from: String, to: String) {
        if !self.model.iter().any(|(a, b)| *a == from && *b == to) {
            self.status = format!("proposed edge {from} → {to}");
            self.added.push((from.clone(), to.clone()));
            self.model.push((from, to));
        }
    }

    fn remove_nodes(&mut self, nodes: &[String]) {
        let before = self.model.len();
        self.model
            .retain(|(a, b)| !nodes.contains(a) && !nodes.contains(b));
        self.status = format!(
            "removed {} node(s), {} edge(s)",
            nodes.len(),
            before - self.model.len()
        );
    }
}

// ---------------------------------------------------------------------------

fn main() -> io::Result<()> {
    let args = parse_args();
    let (model, raw) = match (&args.anvil_snap, &args.path) {
        (Some(root), _) => {
            let raw = load_anvil_snapshot(root)?;
            (crate_level(&raw), Some(raw))
        }
        (None, Some(path)) => (load_json_edges(path)?, None),
        // no source given: use the daemon's snapshot for the current repo
        (None, None) => {
            let raw = load_anvil_snapshot(".")?;
            (crate_level(&raw), Some(raw))
        }
    };

    let mut stack = vec![View::All];
    if let Some(f) = args.focus.clone() {
        stack.push(View::Focus(f));
    }
    if let Some(i) = args.internals.clone() {
        stack.push(View::Internals(i));
    }
    let mut app = App {
        model,
        raw,
        stack,
        selected: None,
        added: Vec::new(),
        status: String::new(),
    };
    let mut flow = app.build_flow()?;
    if args.fit {
        flow.request_fit_view();
    }

    if let Some((w, h)) = args.snapshot {
        headless_snapshot(&app, &mut flow, w, h, args.zoom_read.as_deref());
        return Ok(());
    }

    // `ratatui::run` enables raw mode + alternate screen but NOT mouse
    // reporting — without this the terminal never sends mouse events at all
    // (looks like "mouse doesn't work", especially over tmux/ssh).
    let _mouse = MouseCaptureGuard::enable();
    ratatui::run(|terminal| run_app(terminal, &mut app, &mut flow, args.fit))
}

/// Turns mouse reporting on for the lifetime of the interactive session and
/// reliably off again on any exit path (q, error, panic unwind).
struct MouseCaptureGuard;

impl MouseCaptureGuard {
    fn enable() -> Self {
        let _ = crossterm::execute!(io::stdout(), EnableMouseCapture);
        Self
    }
}

impl Drop for MouseCaptureGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(io::stdout(), DisableMouseCapture);
    }
}

fn draw(frame: &mut ratatui::Frame, app: &App, flow: &mut Flow) {
    let [graph_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    frame.render_widget(Background::new(flow), graph_area);
    frame.render_widget(&mut *flow, graph_area);

    let edges = app.current_edges();
    let status = Line::from(vec![
        format!(" {} ", app.breadcrumb()).bold(),
        format!("│ {} nodes {} edges ", node_count(&edges), edges.len()).into(),
        if app.added.is_empty() {
            "".into()
        } else {
            format!("│ +{} proposed ", app.added.len()).yellow()
        },
        format!("│ {} ", app.status).dim(),
        "│ ↑↓←→ select · z read · +/- zoom · 0 1:1 · Enter drill · i internals · Esc back · f fit · q quit"
            .dim(),
    ]);
    frame.render_widget(
        Paragraph::new(status).style(Style::new().on_black()),
        status_area,
    );
}

fn node_count(edges: &[(String, String)]) -> usize {
    let mut set = HashSet::new();
    for (a, b) in edges {
        set.insert(a);
        set.insert(b);
    }
    set.len()
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    flow: &mut Flow,
    fit: bool,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app, flow))?;

        let mut rebuild = false;
        match event::read()? {
            CrosstermEvent::Key(key) => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('z') => {
                    // zoom-to-read: centre the selection (if any), snap to 1:1
                    // so node labels are legible even in a dense graph
                    flow.center_on_selected();
                    flow.zoom_to(1.0);
                    app.status = "zoom 1:1".into();
                }
                KeyCode::Enter => {
                    if let Some(node) = app.selected.clone() {
                        rebuild = app.push_view(View::Focus(node));
                    }
                }
                KeyCode::Char('i') => {
                    // internals of the selected crate, or of the focused one
                    let target = app.selected.clone().or(match app.stack.last() {
                        Some(View::Focus(f)) => Some(f.clone()),
                        _ => None,
                    });
                    if let (Some(krate), true) = (target, app.raw.is_some()) {
                        rebuild = app.push_view(View::Internals(krate));
                    }
                }
                KeyCode::Esc | KeyCode::Char('b') => rebuild = app.back(),
                _ => {
                    // controls layer first (+/- zoom, 0 reset, f fit), then
                    // the flow layer (arrows select, hjkl pan, c centre, Del)
                    let resp = flow.handle_controls_key_event(key);
                    let resp = if matches!(resp, rataflow::EventResponse::NotHandled) {
                        flow.handle_key_event(key)
                    } else {
                        resp
                    };
                    for ev in resp.into_events() {
                        rebuild |= apply_event(app, flow, ev);
                    }
                }
            },
            CrosstermEvent::Mouse(mouse) => {
                for ev in flow.handle_mouse_event(mouse).into_events() {
                    rebuild |= apply_event(app, flow, ev);
                }
            }
            CrosstermEvent::Resize(_, _) => flow.request_fit_view(),
            _ => {}
        }

        if rebuild {
            match app.build_flow() {
                Ok(new_flow) => {
                    *flow = new_flow;
                    if fit {
                        flow.request_fit_view();
                    }
                }
                Err(e) => {
                    // e.g. drilled into a view with no edges — undo and report
                    app.back();
                    app.status = format!("cannot open view: {e}");
                }
            }
        }
    }
}

/// Apply a semantic flow event to the app model. Returns true when the flow
/// widget must be rebuilt from the model.
fn apply_event(app: &mut App, flow: &mut Flow, ev: FlowEvent) -> bool {
    match ev {
        FlowEvent::NodeClicked { node_id } => {
            app.status = format!("selected {node_id}");
            app.selected = Some(node_id);
            false
        }
        FlowEvent::SelectionChanged { node_ids, .. } => {
            if let Some(id) = node_ids.first() {
                app.selected = Some(id.clone());
            }
            false
        }
        FlowEvent::ConnectionCompleted(conn) => {
            let (from, to) = (conn.source.clone(), conn.target.clone());
            flow.add_edge_from_connection(conn, StepEdge::default());
            app.add_edge(from, to);
            false
        }
        FlowEvent::Deleted { node_ids, .. } => {
            let nodes: Vec<String> = node_ids.iter().map(ToString::to_string).collect();
            if nodes.is_empty() {
                false
            } else {
                app.remove_nodes(&nodes);
                true
            }
        }
        _ => false,
    }
}

/// Render one frame headlessly and print the buffer as plain text.
/// `zoom_read` mimics the interactive `z` key: a first frame establishes the
/// canvas size (and consumes any fit-view request), then the selection is
/// centred at 1:1 and a second frame is captured.
fn headless_snapshot(app: &App, flow: &mut Flow, width: u16, height: u16, zoom_read: Option<&str>) {
    use ratatui::{Terminal, backend::TestBackend};

    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("infallible");
    terminal
        .draw(|frame| draw(frame, app, flow))
        .expect("infallible");
    if let Some(node) = zoom_read {
        flow.select_node(node);
        flow.center_on_selected();
        flow.zoom_to(1.0);
        terminal
            .draw(|frame| draw(frame, app, flow))
            .expect("infallible");
    }

    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        println!("{}", line.trim_end());
    }
}

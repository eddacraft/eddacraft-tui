//! Spike: anvil boundary graph rendered with rataflow, fed from the anvil
//! daemon's warm graph-cache snapshot.
//!
//! Validates rataflow (node-based flow graphs for ratatui) as a candidate for
//! an interactive boundary/impact view in the anvil TUI, and proves the warm
//! graph-cache snapshot can serve as its data source without daemon changes.
//!
//! Beyond rendering, this iteration explores the *write side* of an impact
//! view: what editing the graph should actually mean. Gestures never touch
//! code — they record **intent**, persisted to `.anvil/impact-notes.json`
//! (local runtime state per ADR-073), and the view reconciles intent against
//! reality on every load:
//!
//! ```text
//! gesture            records                reconciliation over time
//! ─────────────────  ─────────────────────  ─────────────────────────────────
//! ! flag + note      "why is this here?"    resurfaces until unflagged
//! n planned node     architecture-to-be     "pending" → "now real ✓"
//! x retire node      should go away         "still present" → "gone ✓"
//! drag edge          intended dependency    "pending" → "landed ✓"
//! ```
//!
//! The same store is scriptable for agents and CI:
//! ```text
//! spike-flow --flag anvil-spike --note "why is this in the prod graph?"
//! spike-flow --plan anvil-impact-view --note "IMPV-001"
//! spike-flow --propose anvil-tui:anvil-impact-view
//! spike-flow --retire some-crate
//! spike-flow --report        # deterministic drift report, always exit 0
//! ```
//!
//! `--policy <file>` overlays boundary rules — a crate-level mirror of
//! anvil-architecture's layer model (member patterns + `depends_on`).
//! Actual edges that cross layers illegally are reported as `⚠` lines and
//! counted in the status bar; unassigned crates are never judged. With a
//! policy loaded, `p` (or `--boundaries`) switches to the **boundaries
//! view**: each layer drawn as a titled container box (rataflow parent
//! nodes), members gridded inside, layers stacked dependents-above-
//! dependencies, and violating edges animated. The productised version
//! would read the real policy engine instead of a sidecar file.
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
//! Enter drill · i internals · Esc/b back · hjkl pan · f fit
//! !            flag the selected node (type a note, Enter; empty = unflag)
//! n            add a planned node (type a name, Enter)
//! x / Delete   toggle retire-intent on the selection (un-plans planned nodes)
//! t            toggle the intent/drift report overlay
//! p            boundaries view: policy layers as container boxes
//! drag on ●    propose a dependency edge · q quit
//! ```
//!
//! Node markers: `⚑` flagged · `◌` planned (not yet real) · `✕` retire intent.
//! Report symbols: `⇢` proposed edge · `⊘` retired edge · `⚠` policy violation.
//!
//! `--snapshot [WxH]` renders one frame to a headless buffer and prints it;
//! `--zoom-read <node>` does the same after centring that node at 1:1.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
};
use rataflow::{Background, Flow, FlowEvent, Sugiyama};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};
use serde::{Deserialize, Serialize};

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
    notes: Option<String>,
    policy: Option<String>,
    note: Option<String>,
    flag: Option<String>,
    plan: Option<String>,
    retire: Option<String>,
    propose: Option<String>,
    report: bool,
    boundaries: bool,
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
        notes: None,
        policy: None,
        note: None,
        flag: None,
        plan: None,
        retire: None,
        propose: None,
        report: false,
        boundaries: false,
        path: None,
    };
    let mut it = std::env::args().skip(1).peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--focus" => args.focus = it.next(),
            "--internals" => args.internals = it.next(),
            "--no-fit" => args.fit = false,
            "--anvil-snap" => args.anvil_snap = it.next(),
            "--zoom-read" => args.zoom_read = it.next(),
            "--notes" => args.notes = it.next(),
            "--policy" => args.policy = it.next(),
            "--note" => args.note = it.next(),
            "--flag" => args.flag = it.next(),
            "--plan" => args.plan = it.next(),
            "--retire" => args.retire = it.next(),
            "--propose" => args.propose = it.next(),
            "--report" => args.report = true,
            "--boundaries" => args.boundaries = true,
            "--snapshot" => {
                // dimensions are optional — only consume the next argument
                // when it actually parses as WxH, so `--snapshot --zoom-read x`
                // does not swallow the following flag
                let dims = it.peek().and_then(|s| parse_dims(s));
                if dims.is_some() {
                    it.next();
                }
                args.snapshot = Some(dims.unwrap_or((160, 45)));
            }
            other => args.path = Some(other.to_string()),
        }
    }
    args
}

fn parse_dims(s: &str) -> Option<(u16, u16)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

// ---------------------------------------------------------------------------
// Intent store: what graph edits *mean* — never code changes, always durable
// annotations reconciled against the real graph on the next load.
// ---------------------------------------------------------------------------

/// Edge key helpers: stored as `"from -> to"` so the JSON is greppable.
fn edge_key(from: &str, to: &str) -> String {
    format!("{from} -> {to}")
}

fn split_edge_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(" -> ")
}

/// Persistent annotations. `BTreeMap` keeps serialization deterministic:
/// same intent, same bytes — diffs of the notes file stay reviewable.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Intent {
    version: u32,
    /// node → note: "why is this here?" questions that should not evaporate.
    #[serde(default)]
    flags: BTreeMap<String, String>,
    /// node → note: architecture that does not exist yet.
    #[serde(default)]
    planned: BTreeMap<String, String>,
    /// node → note: things that should go away.
    #[serde(default)]
    retired: BTreeMap<String, String>,
    /// "from -> to" → note: dependencies that are intended to exist.
    #[serde(default)]
    proposed_edges: BTreeMap<String, String>,
    /// "from -> to" → note: dependencies that should be removed.
    #[serde(default)]
    retired_edges: BTreeMap<String, String>,
}

impl Default for Intent {
    fn default() -> Self {
        Self {
            version: 1,
            flags: BTreeMap::new(),
            planned: BTreeMap::new(),
            retired: BTreeMap::new(),
            proposed_edges: BTreeMap::new(),
            retired_edges: BTreeMap::new(),
        }
    }
}

impl Intent {
    fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &PathBuf) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut out = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        out.push('\n');
        std::fs::write(path, out)
    }

    fn is_empty(&self) -> bool {
        self.flags.is_empty()
            && self.planned.is_empty()
            && self.retired.is_empty()
            && self.proposed_edges.is_empty()
            && self.retired_edges.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Policy overlay: a crate-level mirror of anvil-architecture's layer model
// (`Layers`: patterns + depends_on). Loaded from `--policy <file>`:
//
// ```json
// { "layers": {
//     "engine":  { "members": ["anvil-kernel*", "anvil-graph-cache"],
//                  "depends_on": ["types"] },
//     "types":   { "members": ["anvil-kernel-types"], "depends_on": [] }
// } }
// ```
//
// An edge a → b violates the policy when both endpoints are assigned to
// layers, the layers differ, and b's layer is not in a's `depends_on`.
// Unassigned crates are never judged (warnings over blocks, no guessing).
// The productised version reads the real policy engine instead of a sidecar
// file; drawing layers as rataflow parent-container boxes is the identified
// next visual step (needs post-Sugiyama hierarchy positioning).
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct Policy {
    #[serde(default)]
    layers: BTreeMap<String, PolicyLayer>,
}

#[derive(Deserialize, Default)]
struct PolicyLayer {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

impl Policy {
    fn load(path: &str) -> io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(io::Error::other)
    }

    /// First layer whose member pattern matches the crate name. Patterns are
    /// exact names or a trailing-`*` prefix (`anvil-kernel*`).
    fn layer_of(&self, name: &str) -> Option<&str> {
        self.layers.iter().find_map(|(layer, def)| {
            def.members
                .iter()
                .any(|p| match p.strip_suffix('*') {
                    Some(prefix) => name.starts_with(prefix),
                    None => name == p,
                })
                .then_some(layer.as_str())
        })
    }

    /// Edges that cross a boundary illegally, deterministic order.
    fn violating_pairs(&self, edges: &[(String, String)]) -> Vec<(String, String)> {
        let mut out = BTreeSet::new();
        for (a, b) in edges {
            let (Some(la), Some(lb)) = (self.layer_of(a), self.layer_of(b)) else {
                continue;
            };
            if la != lb
                && let Some(def) = self.layers.get(la)
                && !def.depends_on.iter().any(|d| d == lb)
            {
                out.insert((a.clone(), b.clone()));
            }
        }
        out.into_iter().collect()
    }

    /// Boundary violations among the given edges as report lines.
    fn violations(&self, edges: &[(String, String)]) -> Vec<String> {
        self.violating_pairs(edges)
            .into_iter()
            .map(|(a, b)| {
                let la = self.layer_of(&a).unwrap_or("?");
                let lb = self.layer_of(&b).unwrap_or("?");
                format!("⚠ {la} → {lb}: {a} -> {b} (not allowed by policy)")
            })
            .collect()
    }

    /// Layer stacking order for the boundaries view: layers that depend on
    /// others render above what they depend on (longest-chain depth).
    fn layer_order(&self) -> Vec<String> {
        fn depth(
            policy: &Policy,
            layer: &str,
            seen: &mut Vec<String>,
            memo: &mut BTreeMap<String, usize>,
        ) -> usize {
            if let Some(d) = memo.get(layer) {
                return *d;
            }
            if seen.iter().any(|s| s == layer) {
                return 0; // cycle guard: policy files are user input
            }
            seen.push(layer.to_string());
            let d = policy.layers.get(layer).map_or(0, |def| {
                def.depends_on
                    .iter()
                    .map(|dep| depth(policy, dep, seen, memo) + 1)
                    .max()
                    .unwrap_or(0)
            });
            seen.pop();
            memo.insert(layer.to_string(), d);
            d
        }
        let mut memo = BTreeMap::new();
        let mut order: Vec<String> = self.layers.keys().cloned().collect();
        order.sort_by_key(|l| {
            let d = depth(self, l, &mut Vec::new(), &mut memo);
            (std::cmp::Reverse(d), l.clone())
        });
        order
    }
}

/// Deterministic drift report: intent vs the actual graph. Always
/// warnings-over-blocks — this prints, it never fails a build.
fn reconcile(
    actual_nodes: &BTreeSet<String>,
    actual_edges: &[(String, String)],
    intent: &Intent,
) -> Vec<String> {
    let has_edge = |k: &str| {
        split_edge_key(k).is_some_and(|(f, t)| actual_edges.iter().any(|(a, b)| a == f && b == t))
    };
    let mut lines = Vec::new();
    for (node, note) in &intent.flags {
        let gone = if actual_nodes.contains(node) {
            ""
        } else {
            " (node no longer in graph)"
        };
        lines.push(format!("⚑ {node} — {note}{gone}"));
    }
    for (node, note) in &intent.planned {
        let state = if actual_nodes.contains(node) {
            "now real ✓"
        } else {
            "pending"
        };
        lines.push(format!("◌ {node} — {state}  {note}"));
    }
    for (node, note) in &intent.retired {
        let state = if actual_nodes.contains(node) {
            "still present"
        } else {
            "gone ✓"
        };
        lines.push(format!("✕ {node} — {state}  {note}"));
    }
    for (key, note) in &intent.proposed_edges {
        let state = if has_edge(key) {
            "landed ✓"
        } else {
            "pending"
        };
        lines.push(format!("⇢ {key} — {state}  {note}"));
    }
    for (key, note) in &intent.retired_edges {
        let state = if has_edge(key) {
            "still present"
        } else {
            "gone ✓"
        };
        lines.push(format!("⊘ {key} — {state}  {note}"));
    }
    if lines.is_empty() {
        lines
            .push("no intent recorded — flag (!), plan (n), retire (x), or propose an edge".into());
    }
    lines
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
// App model: drill-down stack over the actual graph plus the intent layer
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum View {
    All,
    Focus(String),
    Internals(String),
    /// Policy layers drawn as literal container boxes (needs `--policy`).
    Boundaries,
}

impl View {
    fn name(&self) -> String {
        match self {
            View::All => "all".into(),
            View::Focus(n) => n.clone(),
            View::Internals(n) => format!("{n} internals"),
            View::Boundaries => "boundaries".into(),
        }
    }
}

/// What the status-bar prompt is currently collecting.
enum Prompt {
    FlagNote { node: String },
    PlanName,
}

struct App {
    /// Crate-level *actual* edge set — what the kernel observed.
    model: Vec<(String, String)>,
    /// Raw file-level import data (snapshot mode only) for Internals views.
    raw: Option<RawGraph>,
    stack: Vec<View>,
    selected: Option<String>,
    intent: Intent,
    notes_path: PathBuf,
    policy: Option<Policy>,
    prompt: Option<(Prompt, String)>,
    show_report: bool,
    status: String,
}

impl App {
    /// Actual node set (undecorated), from the real edges only.
    fn actual_nodes(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        for (a, b) in &self.model {
            set.insert(a.clone());
            set.insert(b.clone());
        }
        set
    }

    /// Edges for the current view: actual edges plus intended (proposed)
    /// edges, so planned architecture renders alongside reality.
    fn current_edges(&self) -> Vec<(String, String)> {
        let mut edges = match self.stack.last().unwrap_or(&View::All) {
            View::All | View::Boundaries => self.model.clone(),
            View::Focus(focus) => self
                .model
                .iter()
                .filter(|(a, b)| a == focus || b == focus)
                .cloned()
                .collect(),
            View::Internals(krate) => match &self.raw {
                Some(raw) => return internals_of(raw, krate),
                None => Vec::new(),
            },
        };
        let in_view = |n: &str| match self.stack.last().unwrap_or(&View::All) {
            View::Focus(focus) => n == focus,
            _ => true,
        };
        for key in self.intent.proposed_edges.keys() {
            if let Some((from, to)) = split_edge_key(key)
                && (in_view(from) || in_view(to))
                && !edges.iter().any(|(a, b)| a == from && b == to)
            {
                edges.push((from.to_string(), to.to_string()));
            }
        }
        edges
    }

    /// Marker prefix communicating intent state on the node label.
    /// Retire-intent outranks planned outranks flagged.
    fn decorate(&self, name: &str) -> String {
        if self.intent.retired.contains_key(name) {
            format!("✕ {name}")
        } else if self.intent.planned.contains_key(name) {
            format!("◌ {name}")
        } else if self.intent.flags.contains_key(name) {
            format!("⚑ {name}")
        } else {
            name.to_string()
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
        if self.stack.last() == Some(&View::Boundaries) {
            return self.build_boundaries_flow();
        }
        let edges = self.current_edges();
        if edges.is_empty() {
            return Err(io::Error::other("current view has no edges"));
        }
        let decorated: Vec<(String, String)> = edges
            .iter()
            .map(|(a, b)| (self.decorate(a), self.decorate(b)))
            .collect();
        let pairs: Vec<(&str, &str)> = decorated
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        Flow::from_edges(&pairs, Sugiyama::vertical()).map_err(io::Error::other)
    }

    /// Boundaries view: each policy layer becomes a rataflow parent-container
    /// box (border-titled, non-selectable), its member crates laid out in a
    /// grid inside it — child positions are parent-relative. Layers stack by
    /// dependency depth (dependents above dependencies), unassigned crates in
    /// a trailing container. Edges route between children across containers;
    /// boundary-violating edges are animated so they stand out.
    // grid geometry works in terminal cells: counts are tiny, casts are safe
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn build_boundaries_flow(&self) -> io::Result<Flow> {
        use rataflow::{Edge, Node, StepEdge, TextContent};

        const CELL_H: f64 = 5.0;
        const GAP: f64 = 2.0;

        let policy = self
            .policy
            .as_ref()
            .ok_or_else(|| io::Error::other("boundaries view needs --policy"))?;
        let edges = self.current_edges();
        let mut names: BTreeSet<String> = BTreeSet::new();
        for (a, b) in &edges {
            names.insert(a.clone());
            names.insert(b.clone());
        }

        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for name in &names {
            let layer = policy.layer_of(name).unwrap_or("(unassigned)");
            groups
                .entry(layer.to_string())
                .or_default()
                .push(name.clone());
        }
        let mut order: Vec<String> = policy
            .layer_order()
            .into_iter()
            .filter(|l| groups.contains_key(l))
            .collect();
        if groups.contains_key("(unassigned)") {
            order.push("(unassigned)".into());
        }

        let mut nodes: Vec<Node<TextContent>> = Vec::new();
        let mut y_cursor = 0.0;
        for layer in &order {
            let members = &groups[layer];
            let cols = (members.len() as f64).sqrt().ceil().max(1.0);
            let cell_w = members
                .iter()
                .map(|m| self.decorate(m).chars().count())
                .max()
                .unwrap_or(8) as f64
                + 6.0;
            let cols_u = cols as usize;
            let rows = members.len().div_ceil(cols_u) as f64;
            let width = cols * (cell_w + GAP) + GAP + 2.0;
            let height = rows * (CELL_H + 1.0) + GAP + 3.0;
            let container_id = format!("▣ {layer}");
            nodes.push(
                Node::new(
                    container_id.clone(),
                    (0.0, y_cursor),
                    (width, height),
                    TextContent::new("").with_title(format!(" {layer} ")),
                )
                .with_selectable(false),
            );
            for (i, member) in members.iter().enumerate() {
                let (row, col) = (i / cols_u, i % cols_u);
                nodes.push(
                    Node::new(
                        self.decorate(member),
                        (
                            GAP + 1.0 + col as f64 * (cell_w + GAP),
                            GAP + 1.0 + row as f64 * (CELL_H + 1.0),
                        ),
                        (cell_w, CELL_H),
                        TextContent::from(member.as_str()),
                    )
                    .with_parent(container_id.clone()),
                );
            }
            y_cursor += height + 4.0;
        }

        let flow_edges: Vec<Edge<StepEdge>> = edges
            .iter()
            .map(|(a, b)| Edge::new(edge_key(a, b), self.decorate(a), self.decorate(b)))
            .collect();
        let mut flow = Flow::with_graph(nodes, flow_edges)
            .map_err(|e| io::Error::other(format!("boundaries graph: {e:?}")))?;
        for (a, b) in policy.violating_pairs(&edges) {
            flow.set_edge_animated(&edge_key(&a, &b), true);
        }
        Ok(flow)
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

    fn persist(&mut self) {
        if let Err(e) = self.intent.save(&self.notes_path) {
            self.status = format!("could not save notes: {e}");
        }
    }

    /// Drag gesture: record an intended dependency. Reality is untouched —
    /// the edge shows up as intent and the report tracks whether it lands.
    fn propose_edge(&mut self, from: &str, to: &str) {
        if self.model.iter().any(|(a, b)| a == from && b == to) {
            self.status = format!("{from} → {to} already exists");
            return;
        }
        self.intent
            .proposed_edges
            .insert(edge_key(from, to), String::new());
        self.status = format!("proposed {from} → {to}");
        self.persist();
    }

    /// Delete gesture: intent, not destruction. A planned node is un-planned
    /// (deleting intent deletes the intent); a real node gets retire-intent
    /// toggled and stays visible with a ✕ marker until reality catches up.
    fn toggle_retire(&mut self, name: &str) {
        if self.intent.planned.remove(name).is_some() {
            self.intent
                .proposed_edges
                .retain(|k, _| split_edge_key(k).is_none_or(|(f, t)| f != name && t != name));
            self.status = format!("un-planned {name}");
        } else if self.intent.retired.remove(name).is_some() {
            self.status = format!("cleared retire-intent on {name}");
        } else {
            self.intent.retired.insert(name.to_string(), String::new());
            self.status = format!("marked {name} for retirement");
        }
        self.persist();
    }

    fn set_flag(&mut self, node: &str, text: String) {
        if text.is_empty() {
            self.intent.flags.remove(node);
            self.status = format!("unflagged {node}");
        } else {
            self.intent.flags.insert(node.to_string(), text);
            self.status = format!("flagged {node}");
        }
        self.persist();
    }

    fn add_planned(&mut self, name: String, text: String) {
        self.status = format!("planned {name} — connect it by dragging an edge");
        self.intent.planned.insert(name.clone(), text);
        self.selected = Some(name);
        self.persist();
    }

    /// Status-bar summary of the note attached to a node, if any.
    fn note_for(&self, node: &str) -> Option<&String> {
        self.intent
            .flags
            .get(node)
            .or_else(|| self.intent.planned.get(node))
            .or_else(|| self.intent.retired.get(node))
    }
}

/// Strip a `⚑ ` / `◌ ` / `✕ ` / `▣ ` marker from a rataflow node id,
/// recovering the real node name intent is keyed by.
fn strip_marker(id: &str) -> &str {
    ["⚑ ", "◌ ", "✕ ", "▣ "]
        .iter()
        .find_map(|m| id.strip_prefix(m))
        .unwrap_or(id)
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

    // The notes file lives with the project the graph describes: local anvil
    // runtime state, deliberately gitignored (ADR-073). Productisation would
    // decide what graduates to a shared, committed intent surface.
    let root = args.anvil_snap.clone().unwrap_or_else(|| ".".to_string());
    let notes_path = args.notes.clone().map_or_else(
        || PathBuf::from(root).join(".anvil/impact-notes.json"),
        PathBuf::from,
    );
    let intent = Intent::load(&notes_path);
    let policy = match &args.policy {
        Some(p) => Some(Policy::load(p)?),
        None => None,
    };

    let mut stack = vec![View::All];
    if let Some(f) = args.focus.clone() {
        stack.push(View::Focus(f));
    }
    if let Some(i) = args.internals.clone() {
        stack.push(View::Internals(i));
    }
    if args.boundaries {
        stack.push(View::Boundaries);
    }
    let mut app = App {
        model,
        raw,
        stack,
        selected: None,
        intent,
        notes_path,
        policy,
        prompt: None,
        show_report: false,
        status: String::new(),
    };

    // Scripted intent ops (agent/CI surface): apply, save, report, exit.
    let scripted = args.flag.is_some()
        || args.plan.is_some()
        || args.retire.is_some()
        || args.propose.is_some();
    if scripted {
        let note = args.note.clone().unwrap_or_default();
        if let Some(node) = &args.flag {
            app.set_flag(
                node,
                if note.is_empty() {
                    "flagged".into()
                } else {
                    note.clone()
                },
            );
        }
        if let Some(node) = &args.plan {
            app.add_planned(node.clone(), note.clone());
        }
        if let Some(node) = &args.retire {
            app.toggle_retire(node);
        }
        if let Some(spec) = &args.propose {
            match spec.split_once(':') {
                Some((from, to)) => app.propose_edge(from, to),
                None => app.status = format!("--propose wants from:to, got {spec}"),
            }
        }
        println!("{}", app.status);
    }
    if args.report || scripted {
        if args.report {
            for line in reconcile(&app.actual_nodes(), &app.model, &app.intent) {
                println!("{line}");
            }
            if let Some(policy) = &app.policy {
                for line in policy.violations(&app.model) {
                    println!("{line}");
                }
            }
        }
        return Ok(());
    }

    let mut flow = app.build_flow()?;
    if args.fit {
        flow.request_fit_view();
    }

    if let Some((w, h)) = args.snapshot {
        let zoom_read = args.zoom_read.as_deref().map(|n| app.decorate(n));
        headless_snapshot(&app, &mut flow, w, h, zoom_read.as_deref());
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

    if app.show_report {
        draw_report(frame, app, graph_area);
    }

    let status = if let Some((prompt, input)) = &app.prompt {
        let label = match prompt {
            Prompt::FlagNote { node } => format!(" note for {node}: "),
            Prompt::PlanName => " planned node name: ".to_string(),
        };
        Line::from(vec![
            label.bold().yellow(),
            input.clone().into(),
            "▏".rapid_blink(),
            "  (Enter save · Esc cancel)".dim(),
        ])
    } else {
        let edges = app.current_edges();
        let intent_summary = if app.intent.is_empty() {
            String::new()
        } else {
            format!(
                "│ ⚑{} ◌{} ✕{} ⇢{} ",
                app.intent.flags.len(),
                app.intent.planned.len(),
                app.intent.retired.len(),
                app.intent.proposed_edges.len()
            )
        };
        let note = app
            .selected
            .as_deref()
            .and_then(|n| app.note_for(n))
            .map(|n| format!("│ ✎ {n} "))
            .unwrap_or_default();
        let violations = app
            .policy
            .as_ref()
            .map(|p| p.violations(&app.model).len())
            .filter(|n| *n > 0)
            .map(|n| format!("│ ⚠{n} boundary "))
            .unwrap_or_default();
        Line::from(vec![
            format!(" {} ", app.breadcrumb()).bold(),
            format!("│ {} nodes {} edges ", node_count(&edges), edges.len()).into(),
            intent_summary.yellow(),
            violations.red(),
            note.italic(),
            format!("│ {} ", app.status).dim(),
            "│ ! flag · n plan · x retire · t report · Enter drill · i internals · z read · q quit"
                .dim(),
        ])
    };
    frame.render_widget(
        Paragraph::new(status).style(Style::new().on_black()),
        status_area,
    );
}

/// Intent/drift overlay: the same reconciliation `--report` prints,
/// rendered over the graph.
fn draw_report(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let mut lines = reconcile(&app.actual_nodes(), &app.model, &app.intent);
    if let Some(policy) = &app.policy {
        lines.extend(policy.violations(&app.model));
    }
    let height = (u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2))
    .min(area.height.saturating_sub(2));
    let width = lines
        .iter()
        .map(|l| {
            u16::try_from(l.chars().count())
                .unwrap_or(u16::MAX)
                .saturating_add(4)
        })
        .max()
        .unwrap_or(20)
        .min(area.width.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + 1,
        width,
        height,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
            .block(Block::bordered().title(" intent vs reality (t to close) ")),
        popup,
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
            // Prompt mode captures the keyboard until Enter/Esc.
            CrosstermEvent::Key(key) if app.prompt.is_some() => {
                rebuild = handle_prompt_key(app, key.code);
            }
            CrosstermEvent::Key(key) => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('z') => {
                    // zoom-to-read: centre the selection (if any), snap to 1:1
                    // so node labels are legible even in a dense graph
                    flow.center_on_selected();
                    flow.zoom_to(1.0);
                    app.status = "zoom 1:1".into();
                }
                KeyCode::Char('!') => {
                    if let Some(node) = app.selected.clone() {
                        let seed = app.intent.flags.get(&node).cloned().unwrap_or_default();
                        app.prompt = Some((Prompt::FlagNote { node }, seed));
                    } else {
                        app.status = "select a node to flag".into();
                    }
                }
                KeyCode::Char('n') => app.prompt = Some((Prompt::PlanName, String::new())),
                KeyCode::Char('x') => {
                    if let Some(node) = app.selected.clone() {
                        app.toggle_retire(&node);
                        rebuild = true;
                    } else {
                        app.status = "select a node to retire".into();
                    }
                }
                KeyCode::Char('t') => app.show_report = !app.show_report,
                KeyCode::Char('p') => {
                    if app.policy.is_some() {
                        rebuild = app.push_view(View::Boundaries);
                    } else {
                        app.status = "boundaries view needs --policy <file>".into();
                    }
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
                        rebuild |= apply_event(app, ev);
                    }
                }
            },
            CrosstermEvent::Mouse(mouse) => {
                for ev in flow.handle_mouse_event(mouse).into_events() {
                    rebuild |= apply_event(app, ev);
                }
            }
            CrosstermEvent::Resize(_, _) if fit => flow.request_fit_view(),
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

/// Status-bar prompt editing: type, Backspace, Enter to commit, Esc to
/// cancel. Returns true when the flow widget must be rebuilt.
fn handle_prompt_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char(c) => {
            if let Some((_, input)) = &mut app.prompt {
                input.push(c);
            }
            false
        }
        KeyCode::Backspace => {
            if let Some((_, input)) = &mut app.prompt {
                input.pop();
            }
            false
        }
        KeyCode::Enter => {
            if let Some((prompt, input)) = app.prompt.take() {
                let input = input.trim().to_string();
                match prompt {
                    Prompt::FlagNote { node } => app.set_flag(&node, input),
                    Prompt::PlanName if !input.is_empty() => {
                        app.add_planned(input, String::new());
                    }
                    Prompt::PlanName => app.status = "cancelled".into(),
                }
                true
            } else {
                false
            }
        }
        KeyCode::Esc => {
            app.prompt = None;
            app.status = "cancelled".into();
            false
        }
        _ => false,
    }
}

/// Apply a semantic flow event to the app model. Returns true when the flow
/// widget must be rebuilt from the model.
fn apply_event(app: &mut App, ev: FlowEvent) -> bool {
    match ev {
        FlowEvent::NodeClicked { node_id } => {
            let name = strip_marker(&node_id).to_string();
            app.status = format!("selected {name}");
            app.selected = Some(name);
            false
        }
        FlowEvent::SelectionChanged { node_ids, .. } => {
            if let Some(id) = node_ids.first() {
                app.selected = Some(strip_marker(id).to_string());
            }
            false
        }
        FlowEvent::ConnectionCompleted(conn) => {
            let from = strip_marker(&conn.source).to_string();
            let to = strip_marker(&conn.target).to_string();
            app.propose_edge(&from, &to);
            // rebuild rather than add in place so the intent layer re-renders
            true
        }
        FlowEvent::Deleted { node_ids, .. } => {
            let mut changed = false;
            for id in &node_ids {
                app.toggle_retire(strip_marker(id));
                changed = true;
            }
            changed
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

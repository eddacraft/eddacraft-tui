//! Impact view (IMPV-001): interactive boundary/impact graph of the current
//! repository, rendered with `eddacraft-tui`'s `flow` feature over the warm
//! graph-cache snapshot.
//!
//! Read lenses only in this first pass: crate-level used-import graph, a
//! crate's direct neighbourhood, and a crate's internal module graph.
//! Keyboard and mouse are both first-class: the surface opts into the
//! anvil pointer loop via [`crate::surface::PointerSurface`] (click select,
//! drag pan / node move, wheel zoom), while every action remains reachable
//! from the keyboard. Intent capture and the policy boundaries view are
//! later IMPV items.

pub mod data;
pub mod render;

use std::cell::RefCell;

use eddacraft_tui::flow::{self, Flow, raw};
use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;

pub use data::{ImpactDataError, ImpactGraph};

/// One level of the drill-down stack.
#[derive(Clone, PartialEq, Eq)]
pub enum ImpactView {
    /// Every crate-level edge.
    All,
    /// Direct neighbourhood of one crate.
    Focus(String),
    /// Internal module graph of one crate.
    Internals(String),
}

impl ImpactView {
    fn name(&self) -> String {
        match self {
            Self::All => "all".into(),
            Self::Focus(n) => n.clone(),
            Self::Internals(n) => format!("{n} internals"),
        }
    }
}

/// Impact surface state: either a loaded graph with a drill-down stack, or a
/// named degraded state.
pub struct ImpactState {
    body: ImpactBody,
    status: String,
    should_quit: bool,
    wants_back: bool,
}

enum ImpactBody {
    Loaded(Box<LoadedImpact>),
    /// The graph could not be loaded or rendered; the reason is shown on
    /// screen (IMPV-001: absent, cold, or unrenderable state is named, never
    /// an empty canvas).
    Degraded(ImpactDataError),
}

struct LoadedImpact {
    graph: ImpactGraph,
    stack: Vec<ImpactView>,
    /// The rendered widget; interior-mutable because `Surface::render`
    /// takes `&self` while rataflow renders through `&mut Flow`.
    flow: RefCell<Flow>,
}

impl ImpactState {
    /// Load the impact graph for `root`; failures become a degraded surface
    /// rather than an error, so the TUI always opens and always says why.
    #[must_use]
    pub fn load(root: &std::path::Path) -> Self {
        match ImpactGraph::load(root) {
            Ok(graph) => Self::from_graph(graph),
            Err(err) => Self {
                body: ImpactBody::Degraded(err),
                status: String::new(),
                should_quit: false,
                wants_back: false,
            },
        }
    }

    /// Build from an already-loaded graph (fixtures, tests). The TUI render
    /// budget applies here — an over-budget graph degrades on screen while
    /// the data stays fully available to `--json`/`--no-tui` callers.
    #[must_use]
    pub fn from_graph(graph: ImpactGraph) -> Self {
        let nodes = graph.crate_count();
        if nodes > data::MAX_RENDERABLE_NODES {
            return Self {
                body: ImpactBody::Degraded(ImpactDataError::TooLarge { nodes }),
                status: String::new(),
                should_quit: false,
                wants_back: false,
            };
        }
        let flow = build_flow(&graph.crate_edges);
        Self {
            body: ImpactBody::Loaded(Box::new(LoadedImpact {
                graph,
                stack: vec![ImpactView::All],
                flow: RefCell::new(flow),
            })),
            status: String::new(),
            should_quit: false,
            wants_back: false,
        }
    }

    /// Breadcrumb naming the current drill position.
    #[must_use]
    pub fn breadcrumb(&self) -> String {
        match &self.body {
            ImpactBody::Loaded(loaded) => loaded
                .stack
                .iter()
                .map(ImpactView::name)
                .collect::<Vec<_>>()
                .join(" › "),
            ImpactBody::Degraded(_) => "impact".into(),
        }
    }

    /// Edges for the current view.
    #[must_use]
    pub fn current_edges(&self) -> Vec<(String, String)> {
        match &self.body {
            ImpactBody::Loaded(loaded) => match loaded.stack.last() {
                Some(ImpactView::Focus(f)) => loaded.graph.neighbourhood(f),
                Some(ImpactView::Internals(k)) => loaded.graph.internals(k),
                _ => loaded.graph.crate_edges.clone(),
            },
            ImpactBody::Degraded(_) => Vec::new(),
        }
    }

    pub(crate) fn degraded(&self) -> Option<&ImpactDataError> {
        match &self.body {
            ImpactBody::Degraded(e) => Some(e),
            ImpactBody::Loaded(_) => None,
        }
    }

    pub(crate) fn flow(&self) -> Option<&RefCell<Flow>> {
        match &self.body {
            ImpactBody::Loaded(loaded) => Some(&loaded.flow),
            ImpactBody::Degraded(_) => None,
        }
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    fn selected(&self) -> Option<String> {
        match &self.body {
            ImpactBody::Loaded(loaded) => loaded.flow.borrow().first_selected_node_id(),
            ImpactBody::Degraded(_) => None,
        }
    }

    fn push_view(&mut self, view: ImpactView) {
        let ImpactBody::Loaded(loaded) = &mut self.body else {
            return;
        };
        if loaded.stack.last() == Some(&view) {
            return;
        }
        let edges = match &view {
            ImpactView::Focus(f) => loaded.graph.neighbourhood(f),
            ImpactView::Internals(k) => loaded.graph.internals(k),
            ImpactView::All => loaded.graph.crate_edges.clone(),
        };
        if edges.is_empty() {
            self.status = format!("{} has nothing to show", view.name());
            return;
        }
        self.status = format!("→ {}", view.name());
        loaded.stack.push(view);
        *loaded.flow.borrow_mut() = build_flow(&edges);
    }

    fn pop_view(&mut self) -> bool {
        let ImpactBody::Loaded(loaded) = &mut self.body else {
            return false;
        };
        if loaded.stack.len() <= 1 {
            return false;
        }
        loaded.stack.pop();
        let edges = match loaded.stack.last() {
            Some(ImpactView::Focus(f)) => loaded.graph.neighbourhood(f),
            Some(ImpactView::Internals(k)) => loaded.graph.internals(k),
            _ => loaded.graph.crate_edges.clone(),
        };
        *loaded.flow.borrow_mut() = build_flow(&edges);
        self.status = "back".into();
        true
    }

    fn with_flow(&mut self, f: impl FnOnce(&mut Flow)) {
        if let ImpactBody::Loaded(loaded) = &self.body {
            f(&mut loaded.flow.borrow_mut());
        }
    }

    /// Whether the current view's node ids are crate names (drillable).
    fn in_crate_view(&self) -> bool {
        match &self.body {
            ImpactBody::Loaded(loaded) => {
                !matches!(loaded.stack.last(), Some(ImpactView::Internals(_)))
            }
            ImpactBody::Degraded(_) => false,
        }
    }

    fn nav(&mut self, direction: raw::Direction) {
        self.with_flow(|flow| flow.select_node_in_direction(direction));
        if let Some(sel) = self.selected() {
            self.status = format!("selected {sel}");
        }
    }
}

fn build_flow(edges: &[(String, String)]) -> Flow {
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    // Duplicate-free by construction (BTreeSet-derived); an unexpected
    // upstream rejection degrades to an empty flow rather than a panic.
    let mut flow = flow::themed_from_edges(&pairs, &EddaCraftTheme).unwrap_or_else(|_| Flow::new());
    flow.request_fit_view();
    flow
}

impl crate::surface::PointerSurface for ImpactState {
    /// Mouse goes straight to the flow widget: click selects, drag pans or
    /// moves nodes, the wheel zooms at the cursor. The surface stays
    /// read-only — connection and deletion events from gestures are
    /// deliberately not applied to the model.
    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        let mut selected = None;
        self.with_flow(|flow| {
            for ev in flow.handle_mouse_event(mouse).into_events() {
                if let flow::FlowEvent::NodeClicked { node_id } = ev {
                    selected = Some(node_id);
                }
            }
        });
        if let Some(sel) = selected {
            self.status = format!("selected {sel}");
        }
    }
}

impl eddacraft_tui::surface::Surface for ImpactState {
    fn surface_name(&self) -> &'static str {
        "Impact"
    }

    fn help_text(&self) -> &'static str {
        "click/↑↓←→ select  wheel/+/- zoom  drag pan/move node  enter drill  i internals  z read  0 1:1  f fit  esc back  q quit"
    }

    fn handle_key(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Up => self.nav(raw::Direction::Up),
            Action::Down => self.nav(raw::Direction::Down),
            Action::Left => self.nav(raw::Direction::Left),
            Action::Right => self.nav(raw::Direction::Right),
            // Drill actions only make sense at crate level — in an internals
            // view the selection is a file label, not a crate name.
            Action::Select => {
                if self.in_crate_view()
                    && let Some(sel) = self.selected()
                {
                    self.push_view(ImpactView::Focus(sel));
                }
            }
            Action::Back | Action::Character('b') => {
                if !self.pop_view() {
                    self.wants_back = true;
                }
            }
            Action::Character('i') => {
                if self.in_crate_view()
                    && let Some(sel) = self.selected()
                {
                    self.push_view(ImpactView::Internals(sel));
                }
            }
            Action::Character('z') => {
                if let Some(sel) = self.selected() {
                    self.with_flow(|flow| flow::zoom_to_read(flow, &sel));
                    self.status = "zoom 1:1".into();
                }
            }
            Action::Character('+' | '=') => self.with_flow(Flow::zoom_in),
            Action::Character('-') => self.with_flow(Flow::zoom_out),
            Action::Character('0') => self.with_flow(Flow::reset_zoom),
            Action::Character('f') => self.with_flow(Flow::request_fit_view),
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn should_back(&self) -> bool {
        self.wants_back
    }

    fn reset(&mut self) {
        self.should_quit = false;
        self.wants_back = false;
    }

    fn render(
        &self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        theme: &EddaCraftTheme,
    ) {
        render::render(frame, area, self, theme);
    }
}

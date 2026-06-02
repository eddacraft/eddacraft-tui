//! Dashboard surfaces.
//!
//! - [`list`] — the read-only picker with live spec previews ([`list::DashboardListState`]),
//!   the entry point for `anvil dashboard`.
//! - [`spec`] — renders a saved json-render dashboard spec through the engine
//!   ([`spec::SpecDashboardState`]), with `.anvil/` data binding and refresh.
//! - [`architecture`] / [`drift`] / [`suppressions`] — the fixed native
//!   per-domain dashboards (TDASH).
//!
//! The CLI owns the catalogue of native dashboards and discovers saved specs
//! under `.anvil/dashboards/`; these surfaces render whatever they are handed.

pub mod architecture;
pub mod drift;
pub mod list;
pub mod spec;
pub mod suppressions;

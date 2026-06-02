pub mod app;
pub mod compat;
pub mod dashboard_catalog;
pub mod dashboard_context;
pub(crate) mod fileio;
pub mod migration;
pub mod shell;
pub mod surface;
pub mod surfaces;

/// Re-export the json-render display sanitiser so CLI display paths (which
/// render untrusted spec-derived strings such as file stems) can strip
/// display-hostile characters — control bytes *and* bidi/zero-width codepoints —
/// without reaching across to `eddacraft-tui` directly.
pub use eddacraft_tui::json_render::sanitize;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod widgets;

//! Track 3 governance surfaces.
//!
//! Each submodule covers one surface defined in
//! `plans/specs/2026-04-08-language-and-coverage-design.md` §5.2.
//! Surfaces start at coverage tier T1 (Scanned) — file detection plus
//! content scanning hand-off — and grow structural rules incrementally.
//!
//! See the per-surface roadmaps: `plans/modules/surface-env-files.aps.md`
//! (SURFENV), `plans/modules/surface-sql-migrations.aps.md` (SURFSQL),
//! `plans/modules/surface-github-actions.aps.md` (SURFGHA), and
//! `plans/modules/surface-dockerfile.aps.md` (SURFDOCK).

pub mod dockerfile;
pub mod env;
pub mod github_actions;
pub mod sql;

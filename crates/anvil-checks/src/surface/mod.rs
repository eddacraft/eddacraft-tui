//! Track 3 governance surfaces.
//!
//! Each submodule covers one surface defined in
//! `plans/specs/2026-04-08-language-and-coverage-design.md` §5.2.
//! Surfaces start at coverage tier T1 (Scanned) — file detection plus
//! content scanning hand-off — and grow structural rules incrementally.
//!
//! See `plans/modules/surface-env-files.aps.md` for the SURFENV roadmap and
//! `plans/modules/surface-sql-migrations.aps.md` for the SURFSQL roadmap.

pub mod env;
pub mod sql;

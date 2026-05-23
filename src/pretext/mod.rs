//! # Pretext layout
//!
//! Two-phase text layout engine for terminal rendering, inspired by Cheng
//! Lou's [Pretext](https://github.com/chenglou/pretext) library for the
//! browser.
//!
//! ## Concept
//!
//! Measure text segments once using `unicode-width` and cache the results,
//! then compute line breaks and layout positions through pure arithmetic on
//! subsequent frames — no repeated measurement. This mirrors Pretext's
//! `prepare()` / `layout()` split, adapted for monospace terminal rendering.
//!
//! ## Targets
//!
//! - **Streaming AI output** — wrap positions are pre-computed before tokens
//!   land, eliminating reflow stutter
//! - **Animated layouts** — text flows around moving ASCII shapes in real time
//! - **Masonry-style dashboards** — reflow cleanly as panel dimensions change
//!
//! ## Pairing with `PretextWidget`
//!
//! The [`crate::widgets::pretext::PretextWidget`] wraps this engine in a
//! ratatui [`StatefulWidget`](ratatui::widgets::StatefulWidget). Cache lives
//! on [`crate::widgets::pretext::PretextState`]; the widget itself is
//! zero-sized and cheap to construct each frame.

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value
)]

pub mod exclusion;
pub mod layout;
pub mod prepare;
pub mod segment;
pub mod types;

pub use exclusion::{
    ExclusionShape, ExclusionZone, RowBand, compute_line_widths, compute_row_band,
    compute_row_bands,
};
pub use layout::{LayoutLine, LayoutResult, PositionedWord, layout};
pub use prepare::PreparedText;
pub use segment::{MeasuredWord, measure_words};
pub use types::{CellPos, CellRect};

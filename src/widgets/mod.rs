use std::time::Duration;

use animate_core::Tween;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Widget};

#[cfg(feature = "big-text")]
#[cfg_attr(docsrs, doc(cfg(feature = "big-text")))]
pub mod big_banner;
pub mod confirm;
pub mod container;
pub mod data_table;
pub mod divider;
pub mod editor;
pub mod header;
pub mod help_bar;
#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub mod image_pane;
pub mod log_panel;
pub mod modal;
pub mod overlay;
pub mod parallel_progress;
pub mod pretext;
pub mod progress_bar;
pub mod select;
pub mod spinner;
pub mod status_badge;
pub mod status_bar;
pub mod text_input;
pub mod toast;
pub mod tree;
pub mod wrappers;

/// Animated `f64` value that eases toward its target over [`ANIM_DURATION_MS`].
pub(crate) type AnimatedF64 = Tween<f64>;

/// Animated `u8` value that eases toward its target over [`ANIM_DURATION_MS`].
pub(crate) type AnimatedU8 = Tween<u8>;

/// Default animation duration shared by progress widgets (milliseconds).
pub(crate) const ANIM_DURATION_MS: u64 = 250;

pub(crate) fn animated_f64(initial: f64) -> AnimatedF64 {
    Tween::new(initial)
        .duration(Duration::from_millis(ANIM_DURATION_MS))
        .easing(animate_core::easing::quad_out)
}

pub(crate) fn animated_u8(initial: u8) -> AnimatedU8 {
    Tween::new(initial)
        .duration(Duration::from_millis(ANIM_DURATION_MS))
        .easing(animate_core::easing::quad_out)
}

/// Render an optional block with a border style, returning the inner area.
/// If no block is provided, returns the original area unchanged.
pub(crate) fn render_block(
    block: Option<&Block<'_>>,
    border_style: Style,
    area: Rect,
    buf: &mut Buffer,
) -> Rect {
    if let Some(block) = block {
        let styled = block.clone().border_style(border_style);
        let inner = styled.inner(area);
        styled.render(area, buf);
        inner
    } else {
        area
    }
}

/// Apply [`Modifier::DIM`] to every cell in the intersection of `area` and the
/// buffer. Shared by overlay scrim and the `Disableable` wrapper so dim
/// semantics stay in lockstep across the crate.
pub(crate) fn dim_buffer(area: Rect, buf: &mut Buffer) {
    let clipped = area.intersection(buf.area);
    for y in clipped.y..clipped.y.saturating_add(clipped.height) {
        for x in clipped.x..clipped.x.saturating_add(clipped.width) {
            buf[(x, y)].modifier.insert(Modifier::DIM);
        }
    }
}

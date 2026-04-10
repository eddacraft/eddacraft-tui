use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};

pub mod confirm;
pub mod container;
pub mod divider;
pub mod editor;
pub mod header;
pub mod log_panel;
pub mod parallel_progress;
pub mod progress_bar;
pub mod select;
pub mod spinner;
pub mod status_badge;
pub mod status_bar;
pub mod text_input;

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

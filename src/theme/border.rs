//! The dashed border that marks a boxed state as unknown. Design system 2.0
//! draws unknown in Ghost Grey with a dashed border (design-system upgrade
//! guide §5(b)); the dash is the cue that survives without colour, so it is
//! drawn at every colour depth.

use ratatui::symbols::{border, line};

/// Plain corners with light dashed edges (`┌╌┐`, `╎`), for a box whose state
/// could not be determined. Pass it to [`ratatui::widgets::Block::border_set`].
///
/// # Stability
///
/// **experimental**, like the other 0.6.0 status additions.
pub const DASHED_BORDER: border::Set<'static> = border::Set {
    top_left: line::NORMAL.top_left,
    top_right: line::NORMAL.top_right,
    bottom_left: line::NORMAL.bottom_left,
    bottom_right: line::NORMAL.bottom_right,
    vertical_left: line::LIGHT_DOUBLE_DASHED.vertical,
    vertical_right: line::LIGHT_DOUBLE_DASHED.vertical,
    horizontal_top: line::LIGHT_DOUBLE_DASHED.horizontal,
    horizontal_bottom: line::LIGHT_DOUBLE_DASHED.horizontal,
};

//! Shared test utilities for TUI snapshot testing.
//!
//! Provides a style-aware `buffer_to_string` that serialises both cell
//! symbols *and* style annotations for use in snapshot tests.

pub mod snapshot {
    use ratatui::buffer::Buffer;
    use ratatui::style::{Color, Modifier};

    /// Build a human-readable style annotation for a single cell.
    /// Returns an empty string when the cell has no styling.
    pub fn style_annotation(cell: &ratatui::buffer::Cell) -> String {
        let has_fg = cell.fg != Color::Reset;
        let styled_bg = cell.bg != Color::Reset;
        let has_mod = !cell.modifier.is_empty();

        if !has_fg && !styled_bg && !has_mod {
            return String::new();
        }

        let mut parts: Vec<String> = Vec::new();
        if has_fg {
            parts.push(format!("fg:{}", cell.fg));
        }
        if styled_bg {
            parts.push(format!("bg:{}", cell.bg));
        }
        if cell.modifier.contains(Modifier::BOLD) {
            parts.push("bold".into());
        }
        if cell.modifier.contains(Modifier::DIM) {
            parts.push("dim".into());
        }
        if cell.modifier.contains(Modifier::ITALIC) {
            parts.push("italic".into());
        }
        if cell.modifier.contains(Modifier::UNDERLINED) {
            parts.push("underlined".into());
        }
        if cell.modifier.contains(Modifier::REVERSED) {
            parts.push("reversed".into());
        }
        if cell.modifier.contains(Modifier::CROSSED_OUT) {
            parts.push("crossed_out".into());
        }
        format!("[{}]", parts.join(","))
    }

    /// Serialise a terminal buffer to a string with style annotations.
    ///
    /// Each cell is rendered as `<symbol>[<style>]` where `<style>` includes
    /// foreground colour, background colour, and modifier flags. Cells with
    /// no styling emit only the symbol.
    pub fn buffer_to_string(buf: &Buffer) -> String {
        let area = buf.area;
        let mut output = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = &buf[(x, y)];
                output.push_str(cell.symbol());
                output.push_str(&style_annotation(cell));
            }
            output.push('\n');
        }
        output
    }
}

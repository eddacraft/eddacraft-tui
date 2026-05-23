//! Layered overlay primitive — a stack of render callbacks composed on top of
//! a base surface. Inspired by Cursive's `StackView`/`add_layer`, this is the
//! foundation for modals, toasts, popovers, and command palettes.
//!
//! ```rust,no_run
//! use eddacraft_tui::prelude::*;
//! use eddacraft_tui::widgets::overlay::{Layer, OverlayStack, Placement};
//! use ratatui::Frame;
//! use ratatui::layout::Rect;
//! use ratatui::widgets::{Block, Borders, Paragraph};
//!
//! fn render_overlay(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
//!     OverlayStack::new(theme)
//!         .push(
//!             Layer::new(|f, a, _t| {
//!                 f.render_widget(
//!                     Paragraph::new("Saved!").block(Block::default().borders(Borders::ALL)),
//!                     a,
//!                 );
//!             })
//!             .placement(Placement::Center { width: 20, height: 3 })
//!             .scrim(true),
//!         )
//!         .render_to_frame(frame, area);
//! }
//! ```

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Clear, Widget};

use crate::theme::Theme;
use crate::widgets::dim_buffer;

/// How a [`Layer`] occupies the parent area.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Placement {
    /// Fill the entire area.
    Fill,
    /// Centred with absolute width/height in cells (clamped to area).
    Center { width: u16, height: u16 },
    /// Centred with width/height as percentages of the area (1..=100).
    /// Values outside that range are clamped.
    CenterPercent { width: u8, height: u8 },
}

impl Placement {
    /// Resolve the placement to a concrete sub-rect of `area`.
    #[must_use]
    pub fn resolve(self, area: Rect) -> Rect {
        match self {
            Self::Fill => area,
            Self::Center { width, height } => {
                centered(area, width.min(area.width), height.min(area.height))
            }
            Self::CenterPercent { width, height } => {
                let w = u32::from(width.clamp(1, 100));
                let h = u32::from(height.clamp(1, 100));
                let layer_w = u16::try_from(u32::from(area.width) * w / 100).unwrap_or(area.width);
                let layer_h =
                    u16::try_from(u32::from(area.height) * h / 100).unwrap_or(area.height);
                centered(area, layer_w.max(1), layer_h.max(1))
            }
        }
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

type RenderFn<'a, T> = Box<dyn FnOnce(&mut Frame, Rect, &T) + 'a>;

/// A single overlay layer: a render closure plus placement and scrim flag.
pub struct Layer<'a, T: Theme> {
    placement: Placement,
    scrim: bool,
    render: RenderFn<'a, T>,
}

impl<'a, T: Theme> Layer<'a, T> {
    /// Create a layer that fills the parent area with no scrim.
    pub fn new(render: impl FnOnce(&mut Frame, Rect, &T) + 'a) -> Self {
        Self {
            placement: Placement::Fill,
            scrim: false,
            render: Box::new(render),
        }
    }

    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// When `true`, dim the parent area before rendering this layer to focus
    /// attention on the overlay. Existing content remains legible.
    #[must_use]
    pub fn scrim(mut self, enabled: bool) -> Self {
        self.scrim = enabled;
        self
    }
}

/// Stack of [`Layer`]s rendered in push order on top of an existing base
/// frame. The top of the stack is the last layer pushed.
pub struct OverlayStack<'a, T: Theme> {
    theme: &'a T,
    layers: Vec<Layer<'a, T>>,
}

impl<'a, T: Theme> OverlayStack<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            layers: Vec::new(),
        }
    }

    #[must_use]
    pub fn push(mut self, layer: Layer<'a, T>) -> Self {
        self.layers.push(layer);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Render every layer in order over `area`. Each layer's region is cleared
    /// (so base content does not bleed through). If any layer requests a
    /// scrim, the entire parent area is dimmed once *before* any layers are
    /// drawn, so subsequent layers render at full intensity rather than being
    /// dimmed by their own scrim or by a prior layer's scrim.
    ///
    /// Use this when rendering inside a draw closure that has access to the
    /// [`Frame`]; for stateless [`Widget`] composition prefer
    /// [`OverlayStack::render`] (the trait impl) which renders against the
    /// buffer only.
    pub fn render_to_frame(self, frame: &mut Frame, area: Rect) {
        let want_scrim = self.layers.iter().any(|l| l.scrim);
        if want_scrim {
            dim_buffer(area, frame.buffer_mut());
        }
        for layer in self.layers {
            let layer_area = layer.placement.resolve(area);
            if layer_area.width == 0 || layer_area.height == 0 {
                continue;
            }
            Clear.render(layer_area, frame.buffer_mut());
            (layer.render)(frame, layer_area, self.theme);
        }
    }
}

impl<T: Theme> Widget for OverlayStack<'_, T> {
    /// Buffer-only [`Widget`] impl. Useful for composing `OverlayStack` with
    /// the wrapper widgets (`Hideable`, `Padded`) and ratatui layout helpers
    /// when you only need scrim + clear + a single rendering pass.
    ///
    /// A synthetic [`Frame`] is not available in this code path, so the
    /// per-layer render closures cannot call `frame.render_widget` and are
    /// skipped. For full overlay rendering use
    /// [`OverlayStack::render_to_frame`] from inside a draw closure instead.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let want_scrim = self.layers.iter().any(|l| l.scrim);
        if want_scrim {
            dim_buffer(area, buf);
        }
        for layer in self.layers {
            let layer_area = layer.placement.resolve(area);
            if layer_area.width == 0 || layer_area.height == 0 {
                continue;
            }
            Clear.render(layer_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;
    use ratatui::widgets::{Block, Borders, Paragraph};

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn placement_fill_returns_input_area() {
        let area = Rect::new(0, 0, 80, 24);
        assert_eq!(Placement::Fill.resolve(area), area);
    }

    #[test]
    fn placement_center_centres_within_area() {
        let area = Rect::new(0, 0, 80, 24);
        let resolved = Placement::Center {
            width: 20,
            height: 4,
        }
        .resolve(area);
        assert_eq!(resolved, Rect::new(30, 10, 20, 4));
    }

    #[test]
    fn placement_center_clamps_to_area() {
        let area = Rect::new(0, 0, 10, 5);
        let resolved = Placement::Center {
            width: 100,
            height: 100,
        }
        .resolve(area);
        assert_eq!(resolved, Rect::new(0, 0, 10, 5));
    }

    #[test]
    fn placement_center_percent_uses_proportional_size() {
        let area = Rect::new(0, 0, 100, 20);
        let resolved = Placement::CenterPercent {
            width: 50,
            height: 50,
        }
        .resolve(area);
        assert_eq!(resolved, Rect::new(25, 5, 50, 10));
    }

    #[test]
    fn empty_stack_renders_without_panic() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let stack = OverlayStack::new(&theme);
                assert!(stack.is_empty());
                stack.render_to_frame(frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn layer_renders_over_base_content() {
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let area = frame.area();
                // Base content
                frame.render_widget(Paragraph::new("BASEBASEBASEBASEBASE"), area);
                // Overlay
                OverlayStack::new(&theme)
                    .push(
                        Layer::new(|f, a, _t| {
                            f.render_widget(
                                Paragraph::new("OVR")
                                    .block(Block::default().borders(Borders::NONE)),
                                a,
                            );
                        })
                        .placement(Placement::Center {
                            width: 3,
                            height: 1,
                        }),
                    )
                    .render_to_frame(frame, area);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // Centre-most cell should be from overlay, not base.
        let centre_y = 2;
        let centre_x = 8; // (20-3)/2 ≈ 8
        let symbols: String = (centre_x..centre_x + 3)
            .map(|x| buf[(x, centre_y)].symbol().to_string())
            .collect();
        assert_eq!(symbols, "OVR");
    }

    #[test]
    fn scrim_applies_dim_modifier_to_base_area() {
        let backend = TestBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(Paragraph::new("hello"), area);
                OverlayStack::new(&theme)
                    .push(
                        Layer::new(|_f, _a, _t| {})
                            .placement(Placement::Center {
                                width: 1,
                                height: 1,
                            })
                            .scrim(true),
                    )
                    .render_to_frame(frame, area);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // A cell outside the layer rect (e.g. (0,0)) should be dimmed.
        assert!(buf[(0, 0)].modifier.contains(Modifier::DIM));
    }

    #[test]
    fn double_scrim_does_not_dim_prior_layer_content() {
        // Two layers with scrim(true) used to dim each other in the original
        // implementation. The fix applies scrim once before any layer draws.
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let area = frame.area();
                OverlayStack::new(&theme)
                    .push(
                        Layer::new(|f, a, _t| {
                            f.render_widget(Paragraph::new("AAA"), a);
                        })
                        .placement(Placement::Center {
                            width: 3,
                            height: 1,
                        })
                        .scrim(true),
                    )
                    .push(
                        Layer::new(|f, a, _t| {
                            f.render_widget(Paragraph::new("BBB"), a);
                        })
                        .placement(Placement::Center {
                            width: 3,
                            height: 1,
                        })
                        .scrim(true),
                    )
                    .render_to_frame(frame, area);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // Centre cells belong to the top-most layer (BBB) and must not carry
        // DIM — which would happen if the second scrim dimmed prior layer
        // pixels.
        let centre_x = (20 - 3) / 2;
        for x in centre_x..centre_x + 3 {
            assert!(
                !buf[(x, 2)].modifier.contains(Modifier::DIM),
                "cell ({x},2) was dimmed",
            );
        }
    }

    #[test]
    fn widget_impl_dims_and_clears_without_frame() {
        // Buffer-only path (Widget impl) should still apply scrim and clear
        // layer rects, but without invoking layer render closures.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        for x in 0..20 {
            for y in 0..5 {
                buf[(x, y)].set_symbol("X");
            }
        }
        OverlayStack::new(&theme)
            .push(
                Layer::new(|_f, _a, _t| {})
                    .placement(Placement::Center {
                        width: 4,
                        height: 1,
                    })
                    .scrim(true),
            )
            .render(area, &mut buf);
        // Cells outside the layer rect should be dimmed.
        assert!(buf[(0, 0)].modifier.contains(Modifier::DIM));
        // Cells inside the layer rect were cleared by the Widget impl.
        let inner_x = (20 - 4) / 2;
        assert_eq!(buf[(inner_x, 2)].symbol(), " ");
    }

    #[test]
    fn layers_render_in_push_order() {
        let backend = TestBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let area = frame.area();
                OverlayStack::new(&theme)
                    .push(
                        Layer::new(|f, a, _t| {
                            f.render_widget(Paragraph::new("AAA"), a);
                        })
                        .placement(Placement::Center {
                            width: 3,
                            height: 1,
                        }),
                    )
                    .push(
                        Layer::new(|f, a, _t| {
                            f.render_widget(Paragraph::new("BBB"), a);
                        })
                        .placement(Placement::Center {
                            width: 3,
                            height: 1,
                        }),
                    )
                    .render_to_frame(frame, area);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        // Top of stack (BBB) should win at the centre.
        let symbols: String = (3..6).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert_eq!(symbols, "BBB");
    }
}

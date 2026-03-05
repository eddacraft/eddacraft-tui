pub mod keyboard;
pub mod theme;
pub mod widgets;

pub mod prelude {
    pub use crate::keyboard::{Action, KeyHandler};
    pub use crate::theme::{EddaCraftTheme, Theme};
    pub use crate::widgets::progress_bar::{ProgressBar, ProgressBarState};
    pub use crate::widgets::select::{Select, SelectState};
    pub use crate::widgets::status_bar::StatusBar;
    pub use crate::widgets::text_input::{TextInput, TextInputState};
}

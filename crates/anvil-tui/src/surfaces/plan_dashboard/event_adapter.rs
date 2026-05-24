use eddacraft_tui::keyboard::Action;

use super::PlanDashboardState;

impl PlanDashboardState {
    pub fn handle_dashboard_action(&mut self, action: Action) {
        if self.filter_mode {
            self.handle_filter_action(action);
            return;
        }

        match action {
            Action::Up => self.move_selection_up(),
            Action::Down => self.move_selection_down(),
            Action::Select => {
                self.show_detail = !self.show_detail;
            }
            Action::Character('/') => {
                self.filter_mode = true;
                self.filter_query.clear();
                self.selected_module = 0;
            }
            Action::Character('?') | Action::Toggle => {
                self.show_help = !self.show_help;
            }
            Action::Character('r') => {
                self.rescan_requested = true;
                self.should_quit = true;
            }
            Action::Quit | Action::Back => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_filter_action(&mut self, action: Action) {
        match action {
            Action::Character(c) if c != '/' => {
                self.filter_query.push(c);
                self.selected_module = 0;
            }
            Action::Backspace => {
                self.filter_query.pop();
                self.selected_module = 0;
            }
            Action::Select => {
                self.filter_mode = false;
            }
            Action::Back => {
                self.filter_mode = false;
                self.filter_query.clear();
                self.selected_module = 0;
            }
            Action::Quit => {
                self.filter_query.push('q');
                self.selected_module = 0;
            }
            _ => {}
        }
    }

    fn move_selection_up(&mut self) {
        let total = self.visible_modules().len();
        if total == 0 {
            self.selected_module = 0;
        } else if self.selected_module == 0 {
            self.selected_module = total - 1;
        } else {
            self.selected_module -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        let total = self.visible_modules().len();
        if total == 0 {
            self.selected_module = 0;
        } else {
            self.selected_module = (self.selected_module + 1) % total;
        }
    }
}

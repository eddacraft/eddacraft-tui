use crate::exclusion::ExclusionDemo;
use crate::masonry::MasonryDemo;
use crate::streaming::StreamingDemo;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DemoTab {
    Streaming,
    Exclusion,
    Masonry,
}

impl DemoTab {
    pub fn all() -> &'static [DemoTab] {
        &[DemoTab::Streaming, DemoTab::Exclusion, DemoTab::Masonry]
    }

    pub fn title(&self) -> &'static str {
        match self {
            DemoTab::Streaming => "Streaming",
            DemoTab::Exclusion => "Exclusion",
            DemoTab::Masonry => "Masonry",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            DemoTab::Streaming => 0,
            DemoTab::Exclusion => 1,
            DemoTab::Masonry => 2,
        }
    }
}

pub struct App {
    pub active_tab: DemoTab,
    pub streaming: StreamingDemo,
    pub exclusion: ExclusionDemo,
    pub masonry: MasonryDemo,
    pub should_quit: bool,
    last_area: Rect,
}

impl App {
    pub fn new() -> Self {
        Self {
            active_tab: DemoTab::Streaming,
            streaming: StreamingDemo::new(),
            exclusion: ExclusionDemo::new(),
            masonry: MasonryDemo::new(),
            should_quit: false,
            last_area: Rect::default(),
        }
    }

    pub fn tick(&mut self) {
        match self.active_tab {
            DemoTab::Streaming => self.streaming.tick(),
            DemoTab::Exclusion => {
                self.exclusion
                    .tick(self.last_area.width, self.last_area.height);
            }
            DemoTab::Masonry => {}
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            DemoTab::Streaming => DemoTab::Exclusion,
            DemoTab::Exclusion => DemoTab::Masonry,
            DemoTab::Masonry => DemoTab::Streaming,
        };
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            DemoTab::Streaming => DemoTab::Masonry,
            DemoTab::Exclusion => DemoTab::Streaming,
            DemoTab::Masonry => DemoTab::Exclusion,
        };
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

        // Tab bar
        let titles: Vec<Line> = DemoTab::all()
            .iter()
            .map(|t| Line::from(Span::styled(t.title(), Style::default().fg(Color::White))))
            .collect();

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .title(" eddacraft-tui pretext demos [Tab/1-3] [q]uit [Space]pause [r]eset [+/-]speed ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .select(self.active_tab.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs, chunks[0]);

        self.last_area = chunks[1];

        match self.active_tab {
            DemoTab::Streaming => self.streaming.render(frame, chunks[1]),
            DemoTab::Exclusion => self.exclusion.render(frame, chunks[1]),
            DemoTab::Masonry => self.masonry.render(frame, chunks[1]),
        }
    }
}

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::ToSpan,
    widgets::{BarChart, Block, List, ListItem, Paragraph},
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let outer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(32), Constraint::Max(32)])
            .split(frame.area());

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(Constraint::from_percentages([25, 75]))
            .split(outer_layout[1]);

        frame.render_widget(
            Paragraph::new(
                "Hello, Ratatui!\n\n\
            Created using https://github.com/ratatui/templates\n\
            Press `Esc`, `Ctrl-C` or `q` to stop running.",
            )
            .block(
                Block::bordered().title(
                    "Ratatui simple template"
                        .to_span()
                        .into_centered_line()
                        .bold()
                        .blue(),
                ),
            )
            .centered(),
            outer_layout[0],
        );

        let data = [("A", 20), ("B", 10), ("C", 15), ("D", 25), ("E", 30)];
        frame.render_widget(
            BarChart::default()
                .block(Block::bordered().title("Bar chart".to_span().into_centered_line()))
                .data(&data)
                .bar_width(4)
                .bar_style(Style::default().green())
                .value_style(Style::default().black().on_green()),
            inner_layout[0],
        );

        frame.render_widget(
            List::new([
                ListItem::new("Item 1"),
                ListItem::new("Item 2"),
                ListItem::new("Item 3"),
                ListItem::new("Item 4"),
                ListItem::new("Item 5"),
            ])
            .block(
                Block::bordered()
                    .green()
                    .title("List widget".to_span().into_centered_line()),
            )
            .style(Style::default().white())
            .highlight_style(Style::default().black().on_yellow())
            .highlight_symbol(">> "),
            inner_layout[1],
        )
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}

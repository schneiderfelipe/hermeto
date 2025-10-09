use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::ToSpan,
    widgets::{BarChart, Block, BorderType, List, ListItem, Padding, Paragraph, Row, Table},
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

        let inner_layout_left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(32), Constraint::Min(32)])
            .split(outer_layout[0]);

        let inner_layout_right = Layout::default()
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
                Block::bordered()
                    .title("Ratatui simple template".bold().blue().into_centered_line()),
            )
            .centered(),
            inner_layout_left[0],
        );

        frame.render_widget(
            Table::new(
                [
                    Row::new(["Ábaco", "Bom pra calcular", "20.00"]),
                    Row::new(["Lapiseira", "Bom pra escrever", "10.00"]),
                ],
                Constraint::from_mins([32, 32, 32]),
            )
            .column_spacing(2)
            .style(Style::default().fg(Color::Magenta))
            .header(
                Row::new(["Name", "Description", "Price"])
                    .underlined()
                    .bold(),
            )
            .block(
                Block::bordered()
                    .padding(Padding::uniform(1))
                    .border_type(BorderType::Rounded)
                    .title("Table".bold().into_centered_line()),
            )
            .cell_highlight_style(Style::default().reversed()),
            inner_layout_left[1],
        );

        let data = [("A", 20), ("B", 10), ("C", 15), ("D", 25), ("E", 30)];
        frame.render_widget(
            BarChart::default()
                .block(Block::bordered().title("Bar chart".to_span().into_centered_line()))
                .data(&data)
                .bar_width(4)
                .bar_style(Style::default().green())
                .value_style(Style::default().black().on_green()),
            inner_layout_right[0],
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
            inner_layout_right[1],
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

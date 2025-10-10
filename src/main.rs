use std::io;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame, style::Stylize};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = Tuia::default().run(&mut terminal);
    ratatui::restore();
    Ok(result?)
}

#[derive(Debug, Default)]
struct Tuia {
    is_running: bool,
}

impl Tuia {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        while self.is_running {
            terminal.draw(|frame| self.draw(frame))?;
            if let Some(message) = self.handle_events()? {
                self.handle_message(message);
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget("Hello world!".magenta().slow_blink(), frame.area())
    }

    fn handle_events(&self) -> io::Result<Option<Message>> {
        match event::read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: _,
                kind: KeyEventKind::Press,
                state: _,
            }) => Ok(Some(Message::Quit)),
            _ => Ok(None),
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Quit => self.is_running = false,
        }
    }
}

enum Message {
    Quit,
}

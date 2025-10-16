use std::{sync::mpsc, thread};

use color_eyre::Result;
use crossterm::event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    widgets::{StatefulWidget, Widget},
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = Application::default().run(&mut terminal);
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
struct Application {
    is_running: bool,
    state: TonnetzState,
}

#[derive(Debug)]
enum Message {
    Quit,
}

impl Application {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            if let event::Event::Key(event::KeyEvent {
                code: event::KeyCode::Esc,
                modifiers,
                kind: event::KeyEventKind::Press,
                state,
            }) = event::read()?
            {
                tx.send(Message::Quit)?;
            }
            Ok(())
        });
        while self.is_running {
            terminal.draw(|frame| {
                frame.render_stateful_widget(Tonnetz::default(), frame.area(), &mut self.state);
            })?;
            match rx.recv()? {
                Message::Quit => self.is_running = false,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Tonnetz {}

#[derive(Debug, Default)]
struct TonnetzState {}

impl StatefulWidget for Tonnetz {
    type State = TonnetzState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        "Placeholder for Tonnetz".blue().render(area, buf)
    }
}

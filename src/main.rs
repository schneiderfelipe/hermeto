use std::{sync::mpsc, thread};

use color_eyre::Result;
use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Layout},
    prelude::Backend,
    widgets::Gauge,
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
    progress: u8,
}

#[derive(Debug)]
enum Message {
    Quit,
    Increment,
    Decrement,
}

impl Application {
    fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            loop {
                match event::read()? {
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char('q'),
                        modifiers: _,
                        kind: event::KeyEventKind::Press,
                        state: _,
                    }) => tx.send(Message::Quit)?,
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Up,
                        modifiers: _,
                        kind: event::KeyEventKind::Press,
                        state: _,
                    }) => tx.send(Message::Increment)?,
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Down,
                        modifiers: _,
                        kind: event::KeyEventKind::Press,
                        state: _,
                    }) => tx.send(Message::Decrement)?,
                    _ => (),
                }
            }
        });
        self.is_running = true;
        while self.is_running {
            terminal.draw(|frame| {
                let layout = Layout::vertical(Constraint::from_maxes([1])).split(frame.area());
                frame.render_widget(
                    Gauge::default().ratio((self.progress as f64 / 256_f64).max(0.0)),
                    layout[0],
                )
            })?;
            match rx.recv()? {
                Message::Quit => self.is_running = false,
                Message::Increment => self.progress = self.progress.wrapping_add(1),
                Message::Decrement => self.progress = self.progress.wrapping_sub(1),
            }
        }
        Ok(())
    }
}

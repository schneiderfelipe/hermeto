use std::{sync::mpsc, thread};

use color_eyre::Result;
use crossterm::event;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
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
            loop {
                if let event::Event::Key(event::KeyEvent {
                    code: event::KeyCode::Esc,
                    modifiers,
                    kind: event::KeyEventKind::Press,
                    state,
                }) = event::read()?
                {
                    tx.send(Message::Quit)?;
                }
            }
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
struct TonnetzState {
    keyboard_layout: KeyboardLayout,
}

impl StatefulWidget for Tonnetz {
    type State = TonnetzState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let rows_layout =
            Layout::vertical(Constraint::from_fills(vec![
                1;
                state.keyboard_layout.rows.len()
            ]))
            .split(area);
        let longest_row_len = state
            .keyboard_layout
            .rows
            .iter()
            .map(|row| row.len())
            .max()
            .unwrap_or(0);
        for (n, (row_layout, row)) in rows_layout
            .iter()
            .zip(&state.keyboard_layout.rows)
            .enumerate()
        {
            let keys_layout = Layout::horizontal(Constraint::from_fills(
                [n as u16]
                    .into_iter()
                    .chain(vec![2; longest_row_len])
                    .chain([(state.keyboard_layout.rows.len() - n - 1) as u16]),
            ))
            .split(*row_layout);
            for (key_layout, key) in keys_layout[1..].iter().zip(row) {
                format!("<{key}>").blue().render(*key_layout, buf)
            }
        }
    }
}

#[derive(Debug)]
struct KeyboardLayout {
    rows: Vec<Vec<char>>,
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        let rows = vec![
            vec!['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
            vec!['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
            vec!['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'ç'],
            vec!['z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', ';'],
        ];
        Self { rows }
    }
}

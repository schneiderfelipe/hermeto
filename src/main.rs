use std::{collections::HashSet, sync::mpsc, thread};

use color_eyre::Result;
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{Paragraph, StatefulWidget, Widget},
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )?;
    let result = Application::default().run(&mut terminal);
    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
struct Application {
    is_running: bool,
    tonnetz: Tonnetz,
    state: TonnetzState,
}

#[derive(Debug)]
enum Message {
    Quit,
    Press(char),
    Release(char),
}

impl Application {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;
        let (tx, rx) = mpsc::channel();
        let keyboard_layout = self.tonnetz.keyboard_layout.clone();
        thread::spawn(move || -> Result<()> {
            loop {
                match event::read()? {
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Esc,
                        modifiers,
                        kind: event::KeyEventKind::Press,
                        state,
                    }) => {
                        tx.send(Message::Quit)?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Press,
                        state,
                    }) if keyboard_layout.contains(&key) => {
                        tx.send(Message::Press(key))?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Release,
                        state,
                    }) if keyboard_layout.contains(&key) => {
                        tx.send(Message::Release(key))?;
                    }
                    _ => (),
                }
            }
        });
        while self.is_running {
            terminal.draw(|frame| {
                frame.render_stateful_widget(&self.tonnetz, frame.area(), &mut self.state);
            })?;
            match rx.recv()? {
                Message::Quit => self.is_running = false,
                Message::Press(key) => {
                    self.state.pressed.insert(key);
                }
                Message::Release(key) => {
                    self.state.pressed.remove(&key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Tonnetz {
    keyboard_layout: KeyboardLayout,
}

#[derive(Debug)]
struct TonnetzState {
    pressed: HashSet<char>,
    base_note: u8,
}

impl Default for TonnetzState {
    fn default() -> Self {
        Self {
            pressed: Default::default(),
            base_note: 27,
        }
    }
}

impl StatefulWidget for &Tonnetz {
    type State = TonnetzState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let rows_layout =
            Layout::vertical(Constraint::from_fills(vec![
                1;
                self.keyboard_layout.rows.len()
            ]))
            .split(area);
        let longest_row_len = self
            .keyboard_layout
            .rows
            .iter()
            .map(|row| row.len())
            .max()
            .unwrap_or(0);
        for (n, (row_layout, row)) in rows_layout
            .iter()
            .zip(&self.keyboard_layout.rows)
            .enumerate()
        {
            let keys_layout = Layout::horizontal(Constraint::from_fills(
                [n as u16]
                    .into_iter()
                    .chain(vec![2; longest_row_len])
                    .chain([(self.keyboard_layout.rows.len() - n - 1) as u16]),
            ))
            .split(*row_layout);
            for (k, (key_layout, key)) in keys_layout[1..].iter().zip(row).enumerate() {
                let note_number = state.base_note + n as u8 * 4 + k as u8 * 7;
                let note = match note_number % 12 {
                    0 => "C",
                    1 => "C#/Db",
                    2 => "D",
                    3 => "D#/Eb",
                    4 => "E",
                    5 => "F",
                    6 => "F#/Gb",
                    7 => "G",
                    8 => "G#/Ab",
                    9 => "A",
                    10 => "A#/Bb",
                    11 => "B",
                    _ => unreachable!(),
                };
                Paragraph::new(vec![
                    Line::from(note).bold(),
                    Line::from(format!("{note_number}")),
                    Line::from(format!("<{key}>", key = key.to_uppercase()).fg(
                        if state.pressed.contains(key) {
                            Color::Yellow
                        } else {
                            Color::Blue
                        },
                    )),
                ])
                .centered()
                .render(*key_layout, buf)
            }
        }
    }
}

#[derive(Clone, Debug)]
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

impl KeyboardLayout {
    fn contains(&self, key: &char) -> bool {
        for row in &self.rows {
            if row.contains(key) {
                return true;
            }
        }
        false
    }
}

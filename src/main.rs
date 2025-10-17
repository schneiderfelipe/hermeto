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
    widgets::{Block, Paragraph, Widget},
};
use rodio::{
    OutputStreamBuilder, Source,
    source::{Function, SignalGenerator},
};
use std::{collections::HashSet, sync::mpsc, thread, time::Duration};

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

#[derive(Debug)]
enum Message {
    Quit,
    Press(char),
    Release(char),
}

#[derive(Debug, Default)]
struct Application {
    is_running: bool,
    tonnetz: Tonnetz,
}

impl Application {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;
        let (message_tx, message_rx) = mpsc::channel();
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
                        message_tx.send(Message::Quit)?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Press,
                        state,
                    }) if keyboard_layout.contains(&key) => {
                        message_tx.send(Message::Press(key))?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Release,
                        state,
                    }) if keyboard_layout.contains(&key) => {
                        message_tx.send(Message::Release(key))?;
                    }
                    _ => (),
                }
            }
        });
        let (frequency_tx, frequency_rx) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            let stream_handle = OutputStreamBuilder::open_default_stream()?;
            loop {
                let frequency = frequency_rx.recv()?;
                stream_handle.mixer().add(
                    SignalGenerator::new(
                        stream_handle.config().sample_rate(),
                        frequency,
                        Function::Triangle,
                    )
                    .amplify_normalized(0.2)
                    .take_duration(Duration::from_millis(1000)),
                );
            }
        });
        while self.is_running {
            terminal.draw(|frame| {
                frame.render_widget(&self.tonnetz, frame.area());
            })?;
            match message_rx.recv()? {
                Message::Quit => self.is_running = false,
                Message::Press(key) => {
                    self.tonnetz.pressed.insert(key);
                    frequency_tx.send(self.tonnetz.note(&key).unwrap().frequency())?;
                }
                Message::Release(key) => {
                    self.tonnetz.pressed.remove(&key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Tonnetz {
    keyboard_layout: KeyboardLayout,
    pressed: HashSet<char>,
    base_note: Note,
}

impl Tonnetz {
    fn note(&self, key: &char) -> Option<Note> {
        if let Some((n, k)) = self.keyboard_layout.get_position(key) {
            Some(Note(u8::from(self.base_note) + n as u8 * 4 + k as u8 * 7))
        } else {
            None
        }
    }
}

impl Default for Tonnetz {
    fn default() -> Self {
        Self {
            keyboard_layout: Default::default(),
            pressed: Default::default(),
            base_note: Note(27),
        }
    }
}

impl Widget for &Tonnetz {
    fn render(self, area: Rect, buf: &mut Buffer) {
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
            for (key_layout, key) in keys_layout[1..].iter().zip(row) {
                let note = self.note(key).unwrap();
                Paragraph::new(vec![
                    Line::from({
                        let (s, f) = note.names();
                        let octave = note.octave();
                        if let Some(f) = f {
                            format!("{s}{octave}/{f}{octave}")
                        } else {
                            format!("{s}{octave}")
                        }
                    })
                    .bold(),
                    Line::from(format!("{n}", n = u8::from(note))),
                    Line::from(format!("{f:.3} Hz", f = note.frequency())),
                    Line::from(format!("<{key}>", key = key.to_uppercase()).blue()),
                ])
                .centered()
                .block(Block::bordered())
                .bg(if self.pressed.contains(key) {
                    Color::Black
                } else {
                    Color::Reset
                })
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

    fn get_position(&self, key: &char) -> Option<(usize, usize)> {
        for (n, row) in self.rows.iter().enumerate() {
            for (k, candidate) in row.iter().enumerate() {
                if candidate == key {
                    return Some((n, k));
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct Note(u8);

impl From<Note> for u8 {
    fn from(note: Note) -> Self {
        note.0
    }
}

impl Note {
    fn frequency(&self) -> f32 {
        440.0 * 2_f32.powf((self.0 as f32 - 69.0) / 12.0)
    }

    fn names(&self) -> (&'static str, Option<&'static str>) {
        match self.0 % 12 {
            0 => ("C", None),
            1 => ("C#", Some("Db")),
            2 => ("D", None),
            3 => ("D#", Some("Eb")),
            4 => ("E", None),
            5 => ("F", None),
            6 => ("F#", Some("Gb")),
            7 => ("G", None),
            8 => ("G#", Some("Ab")),
            9 => ("A", None),
            10 => ("A#", Some("Bb")),
            11 => ("B", None),
            _ => unreachable!(),
        }
    }

    fn octave(&self) -> i8 {
        self.0 as i8 / 12 - 1
    }
}

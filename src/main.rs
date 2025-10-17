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
    tonnetz: Tonnetz<4, 7>,
}

impl Application {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.is_running = true;
        let (message_tx, message_rx) = mpsc::channel();
        let keyboard_layout = self.tonnetz.keyboard_layout();
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
                    self.tonnetz.press(key);
                    frequency_tx.send(self.tonnetz.note(&key).unwrap().frequency())?;
                }
                Message::Release(key) => {
                    self.tonnetz.release(&key);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Tonnetz<const N: u8, const K: u8> {
    base_note: Note,
    keyboard_layout: KeyboardLayout,
    pressed: HashSet<char>,
}

impl<const N: u8, const K: u8> Default for Tonnetz<N, K> {
    fn default() -> Self {
        Self {
            base_note: Note(27),
            keyboard_layout: Default::default(),
            pressed: Default::default(),
        }
    }
}

impl<const N: u8, const K: u8> Tonnetz<N, K> {
    fn note(&self, key: &char) -> Option<Note> {
        self.keyboard_layout
            .find(key)
            .map(|(n, k)| Note(n as u8 * N + k as u8 * K + u8::from(self.base_note)))
    }

    fn keyboard_layout(&self) -> KeyboardLayout {
        self.keyboard_layout
    }

    fn press(&mut self, key: char) -> bool {
        self.pressed.insert(key)
    }

    fn release(&mut self, key: &char) -> bool {
        self.pressed.remove(key)
    }

    fn is_pressed(&self, key: &char) -> bool {
        self.pressed.contains(key)
    }
}

impl<const N: u8, const K: u8> Widget for &Tonnetz<N, K> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (n_rows, max_n_keys) = self.keyboard_layout.size();
        let rows_layout = Layout::vertical(Constraint::from_fills(vec![1; n_rows])).split(area);
        self.keyboard_layout
            .rows()
            .zip(&*rows_layout)
            .enumerate()
            .for_each(|(n, (row, row_layout))| {
                let keys_layout = Layout::horizontal(Constraint::from_fills(
                    [n as u16]
                        .into_iter()
                        .chain(vec![2; max_n_keys])
                        .chain([(n_rows - n - 1) as u16]),
                ))
                .split(*row_layout);
                row.zip(&keys_layout[1..]).for_each(|(key, key_layout)| {
                    let note = self.note(&key).unwrap();
                    Paragraph::new(vec![
                        Line::from({
                            let (s, f) = note.names();
                            let octave = note.octave();
                            f.map_or_else(
                                || format!("{s}{octave}"),
                                |f| format!("{s}{octave}/{f}{octave}"),
                            )
                        })
                        .bold(),
                        Line::from(format!("{n}", n = u8::from(note))),
                        Line::from(format!("{f:.3} Hz", f = note.frequency())),
                        Line::from(format!("<{key}>", key = key.to_uppercase()).blue()),
                    ])
                    .centered()
                    .block(Block::bordered())
                    .bg(if self.is_pressed(&key) {
                        Color::Black
                    } else {
                        Color::Reset
                    })
                    .render(*key_layout, buf)
                });
            });
    }
}

#[derive(Clone, Copy, Debug)]
struct KeyboardLayout {
    rows: [[Option<char>; 13]; 4],
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        Self {
            rows: [
                [
                    Some('\''),
                    Some('1'),
                    Some('2'),
                    Some('3'),
                    Some('4'),
                    Some('5'),
                    Some('6'),
                    Some('7'),
                    Some('8'),
                    Some('9'),
                    Some('0'),
                    Some('-'),
                    Some('='),
                ],
                [
                    Some('q'),
                    Some('w'),
                    Some('e'),
                    Some('r'),
                    Some('t'),
                    Some('y'),
                    Some('u'),
                    Some('i'),
                    Some('o'),
                    Some('p'),
                    None,
                    None,
                    None,
                ],
                [
                    Some('a'),
                    Some('s'),
                    Some('d'),
                    Some('f'),
                    Some('g'),
                    Some('h'),
                    Some('j'),
                    Some('k'),
                    Some('l'),
                    Some('ç'),
                    None,
                    None,
                    None,
                ],
                [
                    // Some('\\'),
                    Some('z'),
                    Some('x'),
                    Some('c'),
                    Some('v'),
                    Some('b'),
                    Some('n'),
                    Some('m'),
                    Some(','),
                    Some('.'),
                    Some(';'),
                    None,
                    None,
                    None,
                ],
            ],
        }
    }
}

impl KeyboardLayout {
    fn contains(&self, key: &char) -> bool {
        self.rows().flatten().any(|candidate| &candidate == key)
    }

    fn find(&self, key: &char) -> Option<(usize, usize)> {
        self.rows()
            .enumerate()
            .flat_map(|(n, row)| row.enumerate().map(move |(k, candidate)| (n, k, candidate)))
            .find_map(|(n, k, candidate)| (&candidate == key).then_some((n, k)))
    }

    fn size(&self) -> (usize, usize) {
        (
            self.rows.len(),
            self.rows().map(|row| row.count()).max().unwrap_or(0),
        )
    }

    fn rows(&self) -> impl Iterator<Item = impl Iterator<Item = char>> {
        self.rows
            .iter()
            .map(|row| row.iter().filter_map(|key| *key))
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

use color_eyre::Result;
use core::{
    iter::{once, repeat_n},
    time::Duration,
};
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use either::Either;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::BlockExt,
    style::{Color, Style, Styled, Stylize},
    text::Line,
    widgets::{Block, BorderType, Widget},
};
use rodio::{
    OutputStreamBuilder, Sample, SampleRate, Source,
    source::{Function, SignalGenerator},
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, mpsc},
    thread,
};
use tui_big_text::{BigText, PixelSize};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )?;
    let result = Application::new(Tonnetz::new(Note(21))).run(&mut terminal);
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

#[derive(Debug)]
struct Application<'a> {
    tonnetz: Tonnetz<'a, 4, 7>,
    is_running: bool,
}

impl<'a> Application<'a> {
    fn new(tonnetz: Tonnetz<'a, 4, 7>) -> Self {
        Self {
            tonnetz,
            is_running: false,
        }
    }

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
                    }) if keyboard_layout.contains(key) => {
                        message_tx.send(Message::Press(key))?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Release,
                        state,
                    }) if keyboard_layout.contains(key) => {
                        message_tx.send(Message::Release(key))?;
                    }
                    _ => (),
                }
            }
        });
        let (frequency_tx, frequency_rx) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            let stream_handle = OutputStreamBuilder::open_default_stream()?;
            // TODO: consider using a Sink after we have our own system
            let tape = Tape::new(stream_handle.config().sample_rate());
            let sources_currently_being_played = Arc::clone(&tape.sources_currently_being_played);
            stream_handle.mixer().add(tape);
            loop {
                let frequency = frequency_rx.recv()?;
                sources_currently_being_played.lock().unwrap().push(
                    SignalGenerator::new(
                        stream_handle.config().sample_rate(),
                        frequency,
                        Function::Triangle,
                    )
                    .fade_in(Duration::from_millis(100))
                    .fade_out(Duration::from_millis(500))
                    .take_duration(Duration::from_millis(1000))
                    .amplify_normalized(0.3),
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
                    self.tonnetz
                        .note(key)
                        .iter()
                        .try_for_each(|note| frequency_tx.send(note.frequency()))?;
                }
                Message::Release(key) => {
                    self.tonnetz.release(key);
                }
            }
        }
        Ok(())
    }
}

struct Tape<S> {
    sample_rate: SampleRate,
    sources_currently_being_played: Arc<Mutex<Vec<S>>>,
}

impl<S> Tape<S> {
    fn new(sample_rate: SampleRate) -> Self {
        Self {
            sample_rate,
            sources_currently_being_played: Arc::new(Mutex::new(Vec::default())),
        }
    }
}

impl<S: Iterator<Item = Sample>> Iterator for Tape<S> {
    type Item = Sample;
    fn next(&mut self) -> Option<Self::Item> {
        let mut index = 0;
        let mut total_sample = 0.0;
        while index < self.sources_currently_being_played.lock().unwrap().len() {
            if let Some(sample) = self.sources_currently_being_played.lock().unwrap()[index].next()
            {
                index += 1;
                total_sample += sample;
            } else {
                self.sources_currently_being_played
                    .lock()
                    .unwrap()
                    .swap_remove(index);
            }
        }
        Some(total_sample)
    }
}

impl<S: Iterator<Item = Sample>> Source for Tape<S> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        1
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Debug)]
struct Tonnetz<'a, const N: u8, const K: u8> {
    base_note: Note,
    keyboard_layout: KeyboardLayout,
    pressed: HashSet<char>,
    block: Option<Block<'a>>,
    style: Style,
}

impl<const N: u8, const K: u8> Tonnetz<'_, N, K> {
    fn new(base_note: Note) -> Self {
        Self {
            base_note,
            keyboard_layout: KeyboardLayout::default(),
            pressed: HashSet::default(),
            block: None,
            style: Style::default(),
        }
    }

    fn note(&self, key: char) -> Option<Note> {
        self.keyboard_layout.find(key).map(|(n, k)| {
            let k = k - n / 2; // adjust for the tilt
            Note(
                u8::from(self.base_note)
                    + N * u8::try_from(n).unwrap()
                    + K * u8::try_from(k).unwrap(),
            )
        })
    }

    fn keyboard_layout(&self) -> KeyboardLayout {
        self.keyboard_layout
    }

    fn press(&mut self, key: char) -> bool {
        self.pressed.insert(key)
    }

    fn release(&mut self, key: char) -> bool {
        self.pressed.remove(&key)
    }

    fn is_pressed(&self, key: char) -> bool {
        self.pressed.contains(&key)
    }
}

impl<const N: u8, const K: u8> Widget for &Tonnetz<'_, N, K> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        buf.set_style(area, self.style);
        self.block.render(area, buf);

        let area = self.block.inner_if_some(area);
        let (n_rows, max_n_keys) = self.keyboard_layout.size();
        let rows_layout = Layout::vertical(Constraint::from_fills(repeat_n(1, n_rows))).split(area);
        let tilted_rows_layout = if self
            .keyboard_layout
            .rows()
            .into_iter()
            .enumerate()
            .filter(|(n, _)| n % 2 == 1)
            .all(|(_, row_layout)| row_layout.into_iter().last().is_some_and(|c| c.is_none()))
        {
            Either::Left(rows_layout.iter().enumerate().map(move |(n, row_layout)| {
                Layout::horizontal(match n % 2 {
                    0 => Constraint::from_fills(repeat_n(2, max_n_keys)),
                    1 => Constraint::from_fills(
                        once(1).chain(repeat_n(2, max_n_keys - 1)).chain(once(1)),
                    ),
                    _ => unreachable!(),
                })
                .split(*row_layout)
            }))
        } else {
            Either::Right(rows_layout.iter().enumerate().map(move |(n, row_layout)| {
                Layout::horizontal(match n % 2 {
                    0 => Constraint::from_fills(repeat_n(2, max_n_keys).chain(once(1))),
                    1 => Constraint::from_fills(once(1).chain(repeat_n(2, max_n_keys))),
                    _ => unreachable!(),
                })
                .split(*row_layout)
            }))
        };

        self.keyboard_layout
            .rows()
            .into_iter()
            .zip(tilted_rows_layout)
            .enumerate()
            .for_each(|(n, (row, keys_layout))| {
                row.into_iter()
                    .zip(keys_layout.iter().skip(n % 2))
                    .filter_map(|(key, key_layout)| {
                        key.and_then(|key| self.note(key).map(|note| (note, key, key_layout)))
                    })
                    .for_each(|(note, key, key_layout)| {
                        buf.set_style(area, self.style);
                        KeyCard::new(note, key)
                            .block(self.block.clone().unwrap_or_else(|| {
                                Block::bordered()
                                    .border_type(BorderType::Rounded)
                                    .style(self.style)
                            }))
                            .style(self.style)
                            .bg(if self.is_pressed(key) {
                                Color::Black
                            } else {
                                Color::Reset
                            })
                            .render(*key_layout, buf);
                    });
            });
    }
}

#[derive(Debug)]
struct KeyCard<'a> {
    note: Note,
    key: char,
    block: Option<Block<'a>>,
    style: Style,
}

impl<'a> KeyCard<'a> {
    fn new(note: Note, key: char) -> Self {
        Self {
            note,
            key,
            block: None,
            style: Style::default(),
        }
    }

    fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }
}

impl Styled for KeyCard<'_> {
    type Item = Self;

    fn style(&self) -> Style {
        self.style
    }

    fn set_style<S: Into<Style>>(self, style: S) -> Self::Item {
        self.style(style)
    }
}

impl Widget for &KeyCard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        buf.set_style(area, self.style);
        self.block.render(area, buf);

        let area = self.block.inner_if_some(area);
        let rows_layout =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(2), Constraint::Max(1)])
                .split(area);
        let top_layout =
            Layout::horizontal(Constraint::from_fills([1, 1, 1])).split(rows_layout[0]);
        let bottom_layout =
            Layout::horizontal(Constraint::from_fills([1, 2])).split(rows_layout[2]);

        let (s, f) = self.note.names();
        let octave = self.note.octave();

        buf.set_style(area, self.style);
        Line::from(s)
            .left_aligned()
            .style(
                f.map_or_else(|| self.style.reversed(), |_| self.style)
                    .bold(),
            )
            .render(top_layout[0], buf);

        buf.set_style(area, self.style);
        Line::from(octave.to_string())
            .centered()
            .style(f.map_or_else(|| self.style.reversed(), |_| self.style))
            .render(top_layout[1], buf);

        buf.set_style(area, self.style);
        Line::from(f.unwrap_or(s))
            .right_aligned()
            .style(
                f.map_or_else(|| self.style.reversed(), |_| self.style)
                    .bold(),
            )
            .render(top_layout[2], buf);

        buf.set_style(area, self.style);
        BigText::builder()
            .lines(vec![Line::from(format!("<{}>", self.key.to_uppercase()))])
            .pixel_size(PixelSize::Quadrant)
            .centered()
            .style(self.style.blue())
            .build()
            .render(rows_layout[1], buf);

        buf.set_style(area, self.style);
        Line::from(u8::from(self.note).to_string())
            .left_aligned()
            .style(self.style)
            .render(bottom_layout[0], buf);

        buf.set_style(area, self.style);
        let frequency = self.note.frequency();
        Line::from(format!("{frequency:.3} Hz"))
            .right_aligned()
            .style(
                if 20.0 < frequency || frequency > 20_000.0 {
                    self.style
                } else {
                    self.style.red()
                }
                .dim(),
            )
            .render(bottom_layout[1], buf);
    }
}

#[derive(Clone, Copy, Debug)]
struct KeyboardLayout {
    rows: [[Option<char>; 14]; 4],
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
                    None,
                ],
                [
                    None,
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
                    Some('['),
                    None,
                ],
                [
                    None,
                    None,
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
                    Some(']'),
                ],
                [
                    None,
                    Some('\\'),
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
                ],
            ],
        }
    }
}

impl KeyboardLayout {
    fn contains(&self, key: char) -> bool {
        self.rows()
            .into_iter()
            .flatten()
            .flatten()
            .any(|candidate| candidate == key)
    }

    fn find(&self, key: char) -> Option<(usize, usize)> {
        self.rows()
            .into_iter()
            .enumerate()
            .flat_map(|(n, row)| {
                row.into_iter()
                    .enumerate()
                    .map(move |(k, candidate)| (n, k, candidate))
            })
            .filter_map(|(n, k, candidate)| candidate.map(|candidate| (n, k, candidate)))
            .find_map(|(n, k, candidate)| (candidate == key).then_some((n, k)))
    }

    fn size(&self) -> (usize, usize) {
        (
            self.rows.len(),
            self.rows()
                .into_iter()
                .map(|row| row.len())
                .max()
                .unwrap_or(0),
        )
    }

    fn rows(&self) -> [[Option<char>; 14]; 4] {
        self.rows
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
    fn frequency(self) -> f32 {
        440.0 * 2_f32.powf((f32::from(self.0) - 69.0) / 12.0)
    }

    fn names(self) -> (&'static str, Option<&'static str>) {
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

    fn octave(self) -> i8 {
        i8::try_from(self.0).unwrap() / 12 - 1
    }
}

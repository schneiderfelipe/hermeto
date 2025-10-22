use cli_log::Level;
use color_eyre::Result;
use core::{mem::replace, time::Duration};
use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::{DefaultTerminal, prelude::*, style::Styled, widgets::Block};
use rodio::{
    OutputStreamBuilder, Sample, SampleRate, Source,
    source::{FadeIn, FadeOut, Function, SignalGenerator, TakeDuration},
};
use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};
use tui_big_text::{BigText, PixelSize};

mod tonnetz;
use crate::tonnetz::Tonnetz;

fn main() -> Result<()> {
    color_eyre::install()?;
    cli_log::init_cli_log!();
    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )?;
    let result = Application::new(Tonnetz::new(Note(21))).run(&mut terminal);
    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    ratatui::restore();
    cli_log::log_mem(Level::Info);
    result
}

#[derive(Debug)]
struct CassetteTape {
    controller: CassetteController,
}

impl CassetteTape {
    fn new(sample_rate: SampleRate) -> Self {
        Self {
            controller: CassetteController::new(sample_rate),
        }
    }
}
impl CassetteTape {
    fn controller(&self) -> CassetteController {
        self.controller.clone()
    }
}

impl Iterator for CassetteTape {
    type Item = Sample;
    fn next(&mut self) -> Option<Self::Item> {
        let mut index = 0;
        let mut total_sample = 0.0;
        let mut controller = self.controller.0.lock().unwrap();
        while index < controller.sources.len() {
            if let Some(sample) = controller.sources[index].1.next() {
                index += 1;
                total_sample += sample;
            } else {
                controller.sources.swap_remove(index);
            }
        }
        Some(total_sample)
    }
}

impl Source for CassetteTape {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        1
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.controller.0.lock().unwrap().sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
enum Play {
    On(Note, char),
    Off(char),
}

#[derive(Clone, Debug)]
struct CassetteController(Arc<Mutex<CassetteControllerInner>>);
#[derive(Debug)]
struct CassetteControllerInner {
    sample_rate: SampleRate,
    sources: Vec<(char, FadeOut<FadeIn<TakeDuration<SignalGenerator>>>)>,
}

impl CassetteController {
    fn new(sample_rate: SampleRate) -> Self {
        Self(Arc::new(Mutex::new(CassetteControllerInner {
            sample_rate,
            sources: Vec::default(),
        })))
    }

    fn on(&self, key: char, note: Note) {
        let mut controller = self.0.lock().unwrap();
        match controller
            .sources
            .iter()
            .position(|(candidate, _)| *candidate == key)
        {
            None => {
                let sample_rate = controller.sample_rate;
                controller.sources.push((
                    key,
                    SignalGenerator::new(sample_rate, note.frequency(), Function::Triangle)
                        .take_duration(Duration::from_millis(6_000))
                        .fade_in(Duration::from_millis(60))
                        .fade_out(Duration::from_millis(3_000)),
                ));
            }
            Some(index) => {
                let new_source = controller.sources[index]
                    .1
                    .inner()
                    .clone()
                    .fade_out(Duration::from_millis(3_000));
                let _ = replace(&mut controller.sources[index].1, new_source);
            }
        }
    }

    fn off(&self, key: char) {
        let mut controller = self.0.lock().unwrap();
        if let Some(index) = controller
            .sources
            .iter()
            .position(|(candidate, _)| *candidate == key)
        {
            let new_source = controller.sources[index]
                .1
                .inner()
                .clone()
                .fade_out(Duration::from_millis(60));
            let _ = replace(&mut controller.sources[index].1, new_source);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Message {
    Press(char),
    Release(char),
    Quit,
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Running,
}

#[derive(Debug)]
struct Application<'a> {
    mode: Option<Mode>,
    tonnetz: Tonnetz<'a, 4, 7>,
}

impl<'a> Application<'a> {
    const fn new(tonnetz: Tonnetz<'a, 4, 7>) -> Self {
        Self {
            mode: None,
            tonnetz,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.mode = Some(Mode::Running);
        let (message_tx, message_rx) = mpsc::channel();
        let keyboard_layout = self.tonnetz.keyboard_layout();
        thread::spawn(move || -> Result<()> {
            loop {
                match event::read()? {
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Press,
                        state,
                    }) if keyboard_layout.contains(key) => message_tx.send(Message::Press(key))?,
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Char(key),
                        modifiers,
                        kind: event::KeyEventKind::Release,
                        state,
                    }) if keyboard_layout.contains(key) => {
                        message_tx.send(Message::Release(key))?;
                    }
                    event::Event::Key(event::KeyEvent {
                        code: event::KeyCode::Esc,
                        modifiers,
                        kind: event::KeyEventKind::Press,
                        state,
                    }) => message_tx.send(Message::Quit)?,
                    _ => {}
                }
            }
        });
        let (play_tx, play_rx) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            // TODO: consider using a Sink after we have our own system
            let stream_handle = OutputStreamBuilder::open_default_stream()?;
            let sample_rate = stream_handle.config().sample_rate();
            let tape = CassetteTape::new(sample_rate);
            let controller = tape.controller();
            stream_handle.mixer().add(tape);
            loop {
                let play = play_rx.recv()?;
                match play {
                    Play::On(note, key) => controller.on(key, note),
                    Play::Off(key) => controller.off(key),
                }
            }
        });
        while let Some(mode) = self.mode {
            terminal.draw(|frame| match mode {
                Mode::Running => frame.render_widget(&self.tonnetz, frame.area()),
            })?;
            match message_rx.recv()? {
                Message::Press(key) => {
                    self.tonnetz.press(key);
                    self.tonnetz
                        .note(key)
                        .iter()
                        .try_for_each(|note| play_tx.send(Play::On(*note, key)))?;
                }
                Message::Release(key) => {
                    self.tonnetz.release(key);
                    self.tonnetz
                        .note(key)
                        .iter()
                        .try_for_each(|_| play_tx.send(Play::Off(key)))?;
                }
                Message::Quit => self.mode = None,
            }
        }
        Ok(())
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
    const fn rows(&self) -> [[Option<char>; 14]; 4] {
        self.rows
    }

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
        440.0 * ((f32::from(self.0) - 69.0) / 12.0).exp2()
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

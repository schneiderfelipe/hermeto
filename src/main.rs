use color_eyre::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use itertools::Itertools;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, ToSpan},
    widgets::{Paragraph, Widget},
};
use std::{collections::BTreeSet, io, iter::once};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    )?;
    let result = Tuia::default().run(&mut terminal);
    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)?;
    ratatui::restore();
    Ok(result?)
}

#[derive(Debug, Default)]
struct Tuia {
    notes: BTreeSet<Note>,
    is_running: bool,
}

static FIRST_ROW: phf::OrderedMap<char, Note> = phf::phf_ordered_map! {
    'q' => Note::D,
    'w' => Note::A,
    'e' => Note::E,
    'r' => Note::B,
    't' => Note::Fs,
    'y' => Note::Cs,
    'u' => Note::Gs,
    'i' => Note::Ds,
    'o' => Note::As,
    'p' => Note::F,
};
static SECOND_ROW: phf::OrderedMap<char, Note> = phf::phf_ordered_map! {
    'a' => Note::F,
    's' => Note::C,
    'd' => Note::G,
    'f' => Note::D,
    'g' => Note::A,
    'h' => Note::E,
    'j' => Note::B,
    'k' => Note::Fs,
    'l' => Note::Cs,
};
static THIRD_ROW: phf::OrderedMap<char, Note> = phf::phf_ordered_map! {
    'z' => Note::Gs,
    'x' => Note::Ds,
    'c' => Note::As,
    'v' => Note::F,
    'b' => Note::C,
    'n' => Note::G,
    'm' => Note::D,
};

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
        let areas = Layout::vertical([Constraint::Fill(1), Constraint::Max(1)]).split(frame.area());
        frame.render_widget(&Tonnetz, areas[0]);
        frame.render_widget(
            format!("{:#?}", self.notes).magenta().rapid_blink(),
            areas[1],
        )
    }

    fn handle_events(&self) -> io::Result<Option<Message>> {
        match event::read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: _,
                kind: KeyEventKind::Press,
                state: _,
            }) => Ok(Some(Message::Quit)),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: _,
                kind: KeyEventKind::Press,
                state: _,
            }) => {
                if FIRST_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOn(FIRST_ROW[&c])))
                } else if SECOND_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOn(SECOND_ROW[&c])))
                } else if THIRD_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOn(THIRD_ROW[&c])))
                } else {
                    Ok(None)
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: _,
                kind: KeyEventKind::Release,
                state: _,
            }) => {
                if FIRST_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOff(FIRST_ROW[&c])))
                } else if SECOND_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOff(SECOND_ROW[&c])))
                } else if THIRD_ROW.contains_key(&c) {
                    Ok(Some(Message::NoteOff(THIRD_ROW[&c])))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Quit => self.is_running = false,
            Message::NoteOn(c) => {
                self.notes.insert(c);
            }
            Message::NoteOff(c) => {
                self.notes.remove(&c);
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Quit,
    NoteOn(Note),
    NoteOff(Note),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Note {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

#[derive(Debug, Default)]
struct Tonnetz;

impl Widget for &Tonnetz {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let f = |c: Option<char>| {
            c.map(|c| format!("< {}>", c.to_uppercase()).blue().bold())
                .unwrap_or("   ".into())
        };
        let lines = Layout::vertical(Constraint::from_maxes([1, 1, 1])).split(area);
        Line::from(
            FIRST_ROW
                .keys()
                .map(|c| f(Some(*c)))
                .intersperse(f(None))
                .collect::<Vec<_>>(),
        )
        .render(lines[0], buf);
        Line::from(
            once(f(None))
                .chain(SECOND_ROW.keys().map(|c| f(Some(*c))).intersperse(f(None)))
                .collect::<Vec<_>>(),
        )
        .render(lines[1], buf);
        Line::from(
            once(f(None))
                .chain(once(f(None)))
                .chain(THIRD_ROW.keys().map(|c| f(Some(*c))).intersperse(f(None)))
                .collect::<Vec<_>>(),
        )
        .render(lines[2], buf)
    }
}

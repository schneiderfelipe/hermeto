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
    widgets::{StatefulWidget, Widget},
};
use std::{collections::BTreeMap, fmt::Display, io, iter::once};

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
    state: TonnetzState,
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

    fn draw(&mut self, frame: &mut Frame) {
        let areas = Layout::vertical([Constraint::Fill(1), Constraint::Max(1)]).split(frame.area());
        frame.render_stateful_widget(&Tonnetz, areas[0], &mut self.state);
        frame.render_widget(
            format!("{:#?}", self.state).magenta().rapid_blink(),
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
            }) if self.state.is_allowed(&c) => Ok(Some(Message::NoteOn(c))),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: _,
                kind: KeyEventKind::Release,
                state: _,
            }) if self.state.is_allowed(&c) => Ok(Some(Message::NoteOff(c))),
            _ => Ok(None),
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Quit => self.is_running = false,
            Message::NoteOn(c) => self.state.insert(c),
            Message::NoteOff(c) => self.state.remove(&c),
        }
    }
}

#[derive(Debug)]
enum Message {
    Quit,
    NoteOn(char),
    NoteOff(char),
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

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::C => write!(f, "C"),
            Note::Cs => write!(f, "C#"),
            Note::D => write!(f, "D"),
            Note::Ds => write!(f, "D#"),
            Note::E => write!(f, "E"),
            Note::F => write!(f, "F"),
            Note::Fs => write!(f, "F#"),
            Note::G => write!(f, "G"),
            Note::Gs => write!(f, "G#"),
            Note::A => write!(f, "A"),
            Note::As => write!(f, "A#"),
            Note::B => write!(f, "B"),
        }
    }
}

#[derive(Debug, Default)]
struct Tonnetz;

#[derive(Debug, Default)]
struct TonnetzState(BTreeMap<char, Note>);

impl TonnetzState {
    fn is_allowed(&self, c: &char) -> bool {
        if FIRST_ROW.contains_key(&c) | SECOND_ROW.contains_key(&c) | THIRD_ROW.contains_key(&c) {
            true
        } else {
            false
        }
    }
    fn is_pressed(&self, c: &char) -> bool {
        self.0.contains_key(c)
    }
    fn get(&self, c: &char) -> Option<&Note> {
        self.0.get(c)
    }
    fn insert(&mut self, c: char) {
        if FIRST_ROW.contains_key(&c) {
            self.0.insert(c, FIRST_ROW[&c]);
        } else if SECOND_ROW.contains_key(&c) {
            self.0.insert(c, SECOND_ROW[&c]);
        } else if THIRD_ROW.contains_key(&c) {
            self.0.insert(c, THIRD_ROW[&c]);
        }
    }
    fn remove(&mut self, c: &char) {
        self.0.remove(c);
    }
}

impl StatefulWidget for &Tonnetz {
    type State = TonnetzState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let space = "   ".to_span();
        let f = |c: char| {
            format!("<{: ^2}>", c.to_uppercase().to_string())
                .blue()
                .bold()
        };
        let g = |c: char| {
            format!("<{: ^2}>", state.get(&c).unwrap().to_string())
                .blue()
                .bold()
        };
        let lines = Layout::vertical(Constraint::from_maxes([1, 1, 1])).split(area);
        Line::from(
            FIRST_ROW
                .keys()
                .map(|c| {
                    if state.is_pressed(c) {
                        g(*c).yellow()
                    } else {
                        f(*c)
                    }
                })
                .intersperse(space.clone())
                .collect::<Vec<_>>(),
        )
        .render(lines[0], buf);
        Line::from(
            once(space.clone())
                .chain(
                    SECOND_ROW
                        .keys()
                        .map(|c| {
                            if state.is_pressed(c) {
                                g(*c).yellow()
                            } else {
                                f(*c)
                            }
                        })
                        .intersperse(space.clone()),
                )
                .collect::<Vec<_>>(),
        )
        .render(lines[1], buf);
        Line::from(
            once(space.clone())
                .chain(once(space.clone()))
                .chain(
                    THIRD_ROW
                        .keys()
                        .map(|c| {
                            if state.is_pressed(c) {
                                g(*c).yellow()
                            } else {
                                f(*c)
                            }
                        })
                        .intersperse(space),
                )
                .collect::<Vec<_>>(),
        )
        .render(lines[2], buf)
    }
}

use std::{collections::BTreeSet, io};

use color_eyre::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::{DefaultTerminal, Frame, style::Stylize};

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
        frame.render_widget(
            format!("{:#?}", self.notes).magenta().rapid_blink(),
            frame.area(),
        )
    }

    fn handle_events(&self) -> io::Result<Option<Message>> {
        static KEYS_TO_NOTES: phf::Map<char, Note> = phf::phf_map! {
            'a' => Note::F,
            's' => Note::C,
            'd' => Note::G,
            'f' => Note::D,
            'g' => Note::A,
            'h' => Note::E,
            'j' => Note::B,
            'k' => Note::Fs,
            'l' => Note::Cs,
            'ç' => Note::Gs,

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

            'z' => Note::Gs,
            'x' => Note::Ds,
            'c' => Note::As,
            'v' => Note::F,
            'b' => Note::C,
            'n' => Note::G,
            'm' => Note::D,
            ',' => Note::A,
            '.' => Note::E,
            ';' => Note::B,
        };
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
            }) if KEYS_TO_NOTES.contains_key(&c) => Ok(Some(Message::NoteOn(KEYS_TO_NOTES[&c]))),
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: _,
                kind: KeyEventKind::Release,
                state: _,
            }) if KEYS_TO_NOTES.contains_key(&c) => Ok(Some(Message::NoteOff(KEYS_TO_NOTES[&c]))),
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

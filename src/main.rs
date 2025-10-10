use std::io;

use color_eyre::Result;
use crossterm::event::{self, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::try_init()?;
    let result = Application::default().run(&mut terminal);
    ratatui::try_restore()?;
    Ok(result?)
}

#[derive(Debug, Default)]
struct Application {
    counter: u8,
    is_running: bool,
}

#[derive(Debug)]
enum Message {
    Quit,
    Decrement,
    Increment,
}

impl Application {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.is_running = true;
        while self.is_running {
            terminal.draw(|frame| self.draw(frame))?;
            if let Some(message) = self.handle_events()? {
                self.handle_message(message)
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    fn handle_events(&self) -> io::Result<Option<Message>> {
        match event::read()? {
            event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    event::KeyCode::Char('q') => Ok(Some(Message::Quit)),
                    event::KeyCode::Left => Ok(Some(Message::Decrement)),
                    event::KeyCode::Right => Ok(Some(Message::Increment)),
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Quit => self.is_running = false,
            Message::Decrement => self.counter = self.counter.saturating_sub(1),
            Message::Increment => self.counter = self.counter.saturating_add(1),
        }
    }
}

impl Widget for &Application {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = "Counter".bold();
        let instructions = Line::from(vec![
            "Decrement".into(),
            " ".into(),
            "<Left>".blue().bold(),
            " ".into(),
            "Increment".into(),
            " ".into(),
            "<Right>".blue().bold(),
            " ".into(),
            "Quit".into(),
            " ".into(),
            "<Q>".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.into_centered_line())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);
        let counter_text = Line::from(vec![
            "Value:".into(),
            " ".into(),
            self.counter.to_string().yellow(),
        ]);
        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::*;

    #[test]
    fn test_render() {
        let app = Application::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));

        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines([
            "┏━━━━━━━━━━━━━━━━━━━━Counter━━━━━━━━━━━━━━━━━━━━━┓",
            "┃                    Value: 0                    ┃",
            "┃                                                ┃",
            "┗━━Decrement <Left> Increment <Right> Quit <Q>━━━┛",
        ]);
        let title_style = Style::default().bold();
        let counter_style = Style::default().yellow();
        let key_style = Style::default().blue().bold();
        expected.set_style(Rect::new(21, 0, 7, 1), title_style);
        expected.set_style(Rect::new(28, 1, 1, 1), counter_style);
        expected.set_style(Rect::new(13, 3, 6, 1), key_style);
        expected.set_style(Rect::new(30, 3, 7, 1), key_style);
        expected.set_style(Rect::new(43, 3, 3, 1), key_style);

        assert_eq!(buf, expected)
    }

    #[test]
    fn test_handle_events() {
        let mut app = Application::default();
        app.handle_message(Message::Increment);
        assert_eq!(app.counter, 1);

        app.handle_message(Message::Decrement);
        assert_eq!(app.counter, 0);

        app.handle_message(Message::Quit);
        assert!(!app.is_running);
    }
}

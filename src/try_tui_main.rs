use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use std::io;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App {
        commands: vec!["one".into(), "two".into(), "three".into()],
        ..Default::default()
    };

    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Default)]
pub struct App {
    counter: usize,
    exit: bool,
    commands: Vec<String>,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Up => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            KeyCode::Down => self.increment_counter(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
        if self.counter == self.commands.len() - 1 {
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }

    fn decrement_counter(&mut self) {
        if self.counter == 0 {
            self.counter = self.commands.len() - 1;
        } else {
            self.counter -= 1;
        }
    }

    fn get_value(&self) -> Text<'_> {
        let vals: Vec<Line> = self
            .commands
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if i == self.counter {
                    Line::from(format!("> {}", s).red().bold())
                } else {
                    Line::from(format!("  {}", s).yellow())
                }
            })
            .collect();

        Text::from(vals)
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter app tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = self.get_value();

        Paragraph::new(counter_text).block(block).render(area, buf);
    }
}

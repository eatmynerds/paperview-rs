use std::collections::{HashMap, HashSet};

use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub options: Vec<String>,
    pub selected_indices: HashSet<usize>,
    pub error_message: Option<String>,
    pub paths: HashMap<usize, String>,
    pub current_monitor: Option<usize>,
}

pub enum InputMode {
    Normal,
    Editing,
    PathInput,
}

impl App {
    pub fn new(options: Vec<String>) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Editing,
            options,
            selected_indices: HashSet::new(),
            error_message: None,
            paths: HashMap::new(),
            current_monitor: None,
        }
    }

    fn process_input(&mut self) -> bool {
        self.error_message = None;
        let input = self.input.trim();

        if input.is_empty() {
            self.error_message = Some("No input provided. Press Enter to try again.".to_string());
            return false;
        }

        self.selected_indices.clear();
        for part in input.split(',') {
            match part.trim().parse::<usize>() {
                Ok(index) if index < self.options.len() => {
                    self.selected_indices.insert(index);
                }
                Ok(_) => {
                    self.error_message = Some(format!(
                        "Invalid choice(s): {}. Press Enter to try again (or q to exit).",
                        part.trim()
                    ));
                    return false;
                }
                Err(_) => {
                    self.error_message = Some(
                        "Invalid input format. Use numbers separated by commas. Press Enter to try again (or q to exit).".to_string(),
                    );
                    return false;
                }
            }
        }
        true
    }

    fn start_path_input(&mut self) {
        if let Some(&monitor) = self.selected_indices.iter().next() {
            self.current_monitor = Some(monitor);
            self.input.clear();
            self.input_mode = InputMode::PathInput;
        }
    }

    fn save_path(&mut self) -> bool {
        if let Some(monitor) = self.current_monitor {
            self.paths.insert(monitor, self.input.trim().to_string());
            self.selected_indices.remove(&monitor);

            if let Some(&next_monitor) = self.selected_indices.iter().next() {
                self.current_monitor = Some(next_monitor);
                self.input.clear();
            } else {
                self.input_mode = InputMode::Normal;
                self.current_monitor = None;
                return true;
            }
        }
        false
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<HashMap<usize, String>> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            if self.process_input() {
                                self.start_path_input();
                            }
                        }
                        KeyCode::Char(c) => self.input.push(c),
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Esc => return Ok(self.paths),
                        _ => {}
                    },
                    InputMode::PathInput => match key.code {
                        KeyCode::Enter => {
                            if !self.input.trim().is_empty() {
                                if self.save_path() {
                                    return Ok(self.paths);
                                }
                            } else {
                                self.error_message = Some("Path cannot be empty.".to_string());
                            }
                        }
                        KeyCode::Char(c) => self.input.push(c),
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Esc => return Ok(self.paths),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ]);
        let [input_area, options_area, error_area] = vertical.areas(frame.area());

        let input_title = if let Some(monitor) = self.current_monitor {
            format!(" Enter a bitmap directory path for monitor {} ", monitor)
        } else {
            " Enter monitor numbers (comma-separated): ".to_string()
        };
        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title(input_title));
        frame.render_widget(input, input_area);

        let options: Vec<ListItem> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let style = if self.selected_indices.contains(&i) {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(format!("{i}: {option}"), style)))
            })
            .collect();
        let options = List::new(options).block(Block::bordered().title("Monitors"));
        frame.render_widget(options, options_area);

        if let Some(error) = &self.error_message {
            let error_message = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::bordered().title("Error"));
            frame.render_widget(error_message, error_area);
        }
    }
}

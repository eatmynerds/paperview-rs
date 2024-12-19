use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};
use std::collections::{HashMap, HashSet};

pub struct App {
    pub input_mode: InputMode,
    pub options: Vec<String>,
    pub selected_indices: HashSet<usize>,
    pub highlighted_index: usize,
    pub error_message: Option<String>,
    pub paths: HashMap<usize, String>,
    pub current_monitor: Option<usize>,
}

pub enum InputMode {
    Normal,
    PathInput,
}

impl App {
    pub fn new(options: Vec<String>) -> Self {
        Self {
            input_mode: InputMode::Normal,
            options,
            selected_indices: HashSet::new(),
            highlighted_index: 0,
            error_message: None,
            paths: HashMap::new(),
            current_monitor: None,
        }
    }

    fn start_path_input(&mut self) {
        if let Some(&monitor) = self.selected_indices.iter().next() {
            self.current_monitor = Some(monitor);
            self.input_mode = InputMode::PathInput;
        }
    }

    fn save_path(&mut self, input: &str) -> bool {
        if let Some(monitor) = self.current_monitor {
            self.paths.insert(monitor, input.trim().to_string());
            self.selected_indices.remove(&monitor);

            if let Some(&next_monitor) = self.selected_indices.iter().next() {
                self.current_monitor = Some(next_monitor);
            } else {
                self.input_mode = InputMode::Normal;
                self.current_monitor = None;
                return true;
            }
        }
        false
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<HashMap<usize, String>> {
        let mut input = String::new();
        loop {
            terminal.draw(|frame| self.draw(frame, &input))?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Down => {
                            self.highlighted_index =
                                (self.highlighted_index + 1) % self.options.len();
                        }
                        KeyCode::Up => {
                            if self.highlighted_index == 0 {
                                self.highlighted_index = self.options.len() - 1;
                            } else {
                                self.highlighted_index -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            self.selected_indices.insert(self.highlighted_index);
                            self.start_path_input();
                        }
                        KeyCode::Esc => return Ok(self.paths),
                        _ => {}
                    },
                    InputMode::PathInput => match key.code {
                        KeyCode::Enter => {
                            if !input.trim().is_empty() {
                                if self.save_path(&input) {
                                    return Ok(self.paths);
                                }
                                input.clear();
                            } else {
                                self.error_message = Some("Path cannot be empty.".to_string());
                            }
                        }
                        KeyCode::Char(c) => input.push(c),
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Esc => return Ok(self.paths),
                        _ => {}
                    },
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame, input: &str) {
        let vertical = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ]);
        let [input_area, options_area, error_area] = vertical.areas(frame.area());

        let input_title = if let Some(monitor) = self.current_monitor {
            format!(" Enter a bitmap directory path for monitor {} ", monitor)
        } else {
            " Navigate using arrow keys and press Enter to select a monitor ".to_string()
        };
        let input = Paragraph::new(input)
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
                } else if i == self.highlighted_index {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
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


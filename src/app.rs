//! Application state and logic

use std::path::PathBuf;

use iced::{event, window, Subscription, Task};

use crate::data::{extract_parameters, extract_time_range};
use crate::parser::{parse_log, LogRecord};
use crate::ui::{charts_view, loading_view, welcome_view};

#[derive(Debug, Clone)]
pub enum Message {
    FileDropped(PathBuf),
    OpenFilePicker,
    FileSelected(Option<PathBuf>),
    ParseComplete((PathBuf, Result<Vec<LogRecord>, String>)),
    AddChart(String),
    RemoveChart(usize),
    SetCursorTime(f64),
    WindowResized((iced::window::Id, iced::Size)),
}

pub struct App {
    pub file_path: String,
    pub records: Option<Vec<LogRecord>>,
    pub parameters: Vec<String>,
    pub active_charts: Vec<String>,
    pub cursor_time: Option<f64>,
    pub load_error: Option<String>,
    pub is_loading: bool,
    pub chart_width: f32,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                file_path: String::new(),
                records: None,
                parameters: Vec::new(),
                active_charts: Vec::new(),
                cursor_time: None,
                load_error: None,
                is_loading: false,
                chart_width: 700.0,
            },
            Task::none(),
        )
    }

    pub fn subscription(_state: &App) -> Subscription<Message> {
        let file_drop = event::listen_with(move |event, _status, _window| {
            if let iced::Event::Window(window::Event::FileDropped(path)) = event {
                Some(Message::FileDropped(path))
            } else {
                None
            }
        });
        let resize = window::resize_events().map(Message::WindowResized);
        Subscription::batch([file_drop, resize])
    }

    pub fn apply_parse_result(&mut self, path: PathBuf, result: Result<Vec<LogRecord>, String>) {
        self.is_loading = false;
        match result {
            Ok(records) => {
                self.file_path = path.to_string_lossy().to_string();
                self.parameters = extract_parameters(&records);
                self.records = Some(records.clone());
                self.load_error = None;
                self.active_charts.clear();
                let (min_ts, _) = extract_time_range(&records);
                self.cursor_time = Some(min_ts);
            }
            Err(e) => {
                self.load_error = Some(e);
                self.records = None;
                self.parameters = Vec::new();
                self.active_charts.clear();
            }
        }
    }

    pub fn start_parse(path: PathBuf) -> Task<Message> {
        Task::perform(
            async move {
                let path_clone = path.clone();
                let result = tokio::task::spawn_blocking(move || {
                    parse_log(&path_clone.to_string_lossy()).map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|_| Err("Парсинг прерван".to_string()));
                (path, result)
            },
            Message::ParseComplete,
        )
    }

    pub fn update(state: &mut App, message: Message) -> Task<Message> {
        match message {
            Message::FileDropped(path) => {
                state.is_loading = true;
                state.load_error = None;
                App::start_parse(path)
            }
            Message::OpenFilePicker => Task::perform(
                async {
                    tokio::task::spawn_blocking(|| {
                        rfd::FileDialog::new()
                            .add_filter("ArduPilot log", &["bin"])
                            .set_title("Выберите файл лога")
                            .pick_file()
                    })
                    .await
                    .ok()
                    .flatten()
                },
                Message::FileSelected,
            ),
            Message::FileSelected(path) => {
                if let Some(p) = path {
                    state.is_loading = true;
                    state.load_error = None;
                    App::start_parse(p)
                } else {
                    Task::none()
                }
            }
            Message::ParseComplete((path, result)) => {
                state.apply_parse_result(path, result);
                Task::none()
            }
            Message::AddChart(param) => {
                if !state.active_charts.contains(&param) {
                    state.active_charts.push(param);
                }
                Task::none()
            }
            Message::RemoveChart(idx) => {
                if idx < state.active_charts.len() {
                    state.active_charts.remove(idx);
                }
                Task::none()
            }
            Message::SetCursorTime(t) => {
                state.cursor_time = Some(t);
                Task::none()
            }
            Message::WindowResized((_, size)) => {
                state.chart_width = (size.width - 328.0).max(400.0);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        if self.is_loading {
            return loading_view();
        }
        if self.records.is_none() {
            return welcome_view(self);
        }
        charts_view(self)
    }
}

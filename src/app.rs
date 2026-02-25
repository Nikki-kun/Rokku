use std::path::PathBuf;

use iced::{event, window, Subscription, Task};

use crate::data::{
    extract_parameter_groups, extract_parameters, extract_time_range,
    validate_param_series_and_log, ParamGroup,
};
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
    ParamSearchChanged(String),
    ParamGroupToggled(String),
}

pub struct App {
    pub file_path: String,
    pub records: Option<Vec<LogRecord>>,
    pub parameters: Vec<String>,
    pub parameter_groups: Vec<ParamGroup>,
    pub expanded_groups: std::collections::HashSet<String>,
    pub param_search: String,
    pub active_charts: Vec<String>,
    pub cursor_time: Option<f64>,
    pub load_error: Option<String>,
    pub is_loading: bool,
    pub chart_width: f32,
    pub chart_bounds: Option<(f32, f32)>,
}


impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                file_path: String::new(),
                records: None,
                parameters: Vec::new(),
                parameter_groups: Vec::new(),
                expanded_groups: std::collections::HashSet::new(),
                param_search: String::new(),
                active_charts: Vec::new(),
                cursor_time: None,
                load_error: None,
                is_loading: false,
                chart_width: 700.0,
                chart_bounds: None,
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
        match &result {
            Ok(records) => {
                log::info!("Parsed {} records from {}", records.len(), path.display());
                self.file_path = path.to_string_lossy().to_string();
                self.parameters = extract_parameters(records);
                self.parameter_groups = extract_parameter_groups(records);
                self.records = Some(records.clone());
                self.load_error = None;
                self.active_charts.clear();
                self.expanded_groups.clear();
                let (min_ts, _) = extract_time_range(records);
                self.cursor_time = Some(min_ts);
                if self.chart_bounds.is_none() {
                    let chart_left = 24.0 + 280.0 + 24.0 + 20.0 + 20.0 + 16.0;
                    let chart_width =
                        (self.chart_width - chart_left - 24.0 - 16.0 - 16.0).max(100.0);
                    self.chart_bounds = Some((chart_left, chart_width));
                }
            }
            Err(e) => {
                log::error!("Parse failed: {}", e);
                self.load_error = Some(e.clone());
                self.records = None;
                self.parameters = Vec::new();
                self.parameter_groups = Vec::new();
                self.active_charts.clear();
            }
        }
    }

    pub fn start_parse(path: PathBuf) -> Task<Message> {
        log::info!("Loading log file: {}", path.display());
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
                            .add_filter("ArduPilot / PX4 logs", &["bin", "ulg"])
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
                    if let Some(ref records) = state.records {
                        validate_param_series_and_log(records, &param);
                    }
                    log::debug!("Added chart: {}", param);
                    state.active_charts.push(param);
                }
                Task::none()
            }
            Message::RemoveChart(idx) => {
                if idx < state.active_charts.len() {
                    state.active_charts.remove(idx);
                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::SetCursorTime(t) => {
                state.cursor_time = Some(t);
                Task::none()
            }
            Message::WindowResized((_, size)) => {
                state.chart_width = (size.width - 328.0).max(400.0);
                let chart_left = 24.0 + 280.0 + 24.0 + 20.0 + 20.0 + 16.0;
                let chart_width = (size.width - chart_left - 24.0 - 16.0 - 16.0).max(100.0);
                state.chart_bounds = Some((chart_left, chart_width));
                Task::none()
            }
            Message::ParamSearchChanged(s) => {
                state.param_search = s;
                Task::none()
            }
            Message::ParamGroupToggled(name) => {
                if state.expanded_groups.contains(&name) {
                    state.expanded_groups.remove(&name);
                } else {
                    state.expanded_groups.insert(name);
                }
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

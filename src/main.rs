//! Rokku - ArduPilot log viewer with plotters-iced
//! Styled like Halloy IRC client (Ferra theme)

mod parser;

use iced::{
    event, window,
    widget::{
        button, column, row, text, Column, Container, Scrollable, Space,
    },
    Alignment, Color, Element, Length, Subscription, Task, Theme,
};
use plotters::prelude::*;
use plotters_backend::DrawingBackend;
use plotters_iced::{Chart, ChartWidget};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use parser::{parse_log, LogRecord};

// Halloy Ferra theme colors
fn ferra_container(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(0x24, 0x22, 0x26))),
        border: iced::Border {
            color: iced::Color::from_rgb8(0x4f, 0x47, 0x4d),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn main() -> iced::Result {
    iced::application("Rokku - ArduPilot Log Viewer", App::update, App::view)
        .theme(|_| Theme::Ferra)
        .subscription(App::subscription)
        .antialiasing(true)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
enum Message {
    FileDropped(PathBuf),
    OpenFilePicker,
    FileSelected(Option<PathBuf>),
    /// Результат фонового парсинга: (путь, результат)
    ParseComplete((PathBuf, Result<Vec<LogRecord>, String>)),
}

struct App {
    file_path: String,
    records: Option<Vec<LogRecord>>,
    parameters: Vec<String>,
    load_error: Option<String>,
    is_loading: bool,
}

/// Extract all unique parameters (message_type.field_name) from parsed records.
fn extract_parameters(records: &[LogRecord]) -> Vec<String> {
    let mut params: BTreeSet<String> = BTreeSet::new();
    for r in records {
        for field_name in r.fields.keys() {
            params.insert(format!("{}.{}", r.message_type, field_name));
        }
    }
    params.into_iter().collect()
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                file_path: String::new(),
                records: None,
                parameters: Vec::new(),
                load_error: None,
                is_loading: false,
            },
            Task::none(),
        )
    }

    fn subscription(_state: &App) -> Subscription<Message> {
        event::listen_with(move |event, _status, _window| {
            if let iced::Event::Window(window::Event::FileDropped(path)) = event {
                Some(Message::FileDropped(path))
            } else {
                None
            }
        })
    }

    fn apply_parse_result(&mut self, path: PathBuf, result: Result<Vec<LogRecord>, String>) {
        self.is_loading = false;
        match result {
            Ok(records) => {
                self.file_path = path.to_string_lossy().to_string();
                self.parameters = extract_parameters(&records);
                self.records = Some(records);
                self.load_error = None;
            }
            Err(e) => {
                self.load_error = Some(e);
                self.records = None;
                self.parameters = Vec::new();
            }
        }
    }

    fn start_parse(path: PathBuf) -> Task<Message> {
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

    fn update(state: &mut App, message: Message) -> Task<Message> {
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            button(if self.is_loading {
                "Загрузка..."
            } else {
                "Открыть файл..."
            })
            .on_press_maybe(if self.is_loading {
                None
            } else {
                Some(Message::OpenFilePicker)
            })
            .style(button::secondary),
            if self.is_loading {
                text("Загрузка и парсинг...")
                    .size(14)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xf5, 0xd7, 0x6e)),
                    })
            } else if self.file_path.is_empty() {
                text("Перетащите .bin файл сюда или нажмите «Открыть файл»")
                    .size(14)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xab, 0x8a, 0x79)),
                    })
            } else {
                let display_path = std::path::Path::new(&self.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&self.file_path);
                text(display_path)
                    .size(12)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xb1, 0xb6, 0x95)),
                    })
            },
        ]
        .spacing(15)
        .align_y(Alignment::Center);

        let content = column![
            controls,
            if let Some(ref err) = self.load_error {
                text(err)
                    .size(14)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xe0, 0x6b, 0x75)),
                    })
            } else {
                text("")
            },
            Space::with_height(20),
        ];

        let params_list: Element<Message> = if self.parameters.is_empty() {
            text("Параметры появятся после загрузки лога")
                .size(14)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0xab, 0x8a, 0x79)),
                })
                .into()
        } else {
            let mut list = Column::new().spacing(4).padding(8);
            for p in &self.parameters {
                list = list.push(
                    text(p.as_str())
                        .size(12)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(0xfe, 0xcd, 0xb2)),
                        }),
                );
            }
            Scrollable::new(list)
                .height(Length::Fill)
                .width(Length::Fixed(250.0))
                .into()
        };

        let chart_section: Element<Message> = if let Some(ref records) = self.records {
            if records.is_empty() {
                text("Файл пуст или не содержит данных")
                    .size(16)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xab, 0x8a, 0x79)),
                    })
                    .into()
            } else {
                let log_chart = LogChart::new(records.clone());
                Column::new()
                    .push(
                        text(format!("Загружено {} записей", records.len()))
                            .size(14)
                            .style(|_| iced::widget::text::Style {
                                color: Some(Color::from_rgb8(0xb1, 0xb6, 0x95)),
                            }),
                    )
                    .push(
                        ChartWidget::new(log_chart)
                            .width(Length::Fill)
                            .height(Length::Fixed(400.0)),
                    )
                    .spacing(10)
                    .into()
            }
        } else {
            text("Загрузите ArduPilot Dataflash лог (.bin) для просмотра")
                .size(16)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(0xab, 0x8a, 0x79)),
                })
                .into()
        };

        let data_panel = row![
            column![
                text("Параметры")
                    .size(16)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(0xf5, 0xd7, 0x6e)),
                    }),
                Space::with_height(8),
                params_list,
            ]
            .width(250)
            .spacing(4),
            Space::with_width(20),
            column![chart_section].width(Length::Fill),
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill);

        let main_content = column![content, data_panel]
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(ferra_container)
            .into()
    }
}

/// Chart for displaying log data (timestamp vs numeric field)
struct LogChart {
    data: Vec<(f64, f64)>,
}

impl LogChart {
    fn new(records: Vec<LogRecord>) -> Self {
        let mut by_type: HashMap<String, Vec<&LogRecord>> = HashMap::new();
        for r in &records {
            by_type.entry(r.message_type.clone()).or_default().push(r);
        }

        let data = if let Some(att) = by_type.get("ATT") {
            extract_series(att, "Roll")
                .or_else(|| extract_series(att, "Pitch"))
                .or_else(|| extract_series(att, "Yaw"))
                .map(|(d, _)| d)
                .unwrap_or_default()
        } else if let Some(gps) = by_type.get("GPS") {
            extract_series(gps, "Alt")
                .or_else(|| extract_series(gps, "Lat"))
                .map(|(d, _)| d)
                .unwrap_or_default()
        } else {
            by_type
                .values()
                .find_map(|recs| first_numeric_series(recs).map(|(d, _)| d))
                .unwrap_or_default()
        };

        Self { data }
    }
}

fn extract_series(records: &[&LogRecord], field: &str) -> Option<(Vec<(f64, f64)>, String)> {
    let mut points = Vec::new();
    for r in records {
        if let Some(v) = r.fields.get(field) {
            let val = v.as_f64().or_else(|| v.as_i64().map(|x| x as f64))?;
            points.push((r.timestamp_ns as f64 / 1_000_000.0, val));
        }
    }
    if points.is_empty() {
        None
    } else {
        Some((points, field.to_string()))
    }
}

fn first_numeric_series(records: &[&LogRecord]) -> Option<(Vec<(f64, f64)>, String)> {
    if records.is_empty() {
        return None;
    }
    for (key, _) in &records[0].fields {
        if key != "TimeUS" && key != "timestamp" {
            if let Some((points, _)) = extract_series(records, key) {
                if !points.is_empty() {
                    return Some((points, key.clone()));
                }
            }
        }
    }
    None
}

impl Chart<Message> for LogChart {
    type State = ();

    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        mut chart: plotters_iced::ChartBuilder<DB>,
    ) {
        if self.data.is_empty() {
            return;
        }

        let (min_ts, max_ts): (f64, f64) = self
            .data
            .iter()
            .map(|(t, _)| *t)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| (a.min(t), b.max(t)));

        let (min_val, max_val): (f64, f64) = self
            .data
            .iter()
            .map(|(_, v)| *v)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));

        let val_margin = (max_val - min_val).abs().max(1.0) * 0.1;
        let val_range = (min_val - val_margin)..(max_val + val_margin);

        let mut chart = chart
            .x_label_area_size(30)
            .y_label_area_size(40)
            .margin(20)
            .build_cartesian_2d(min_ts..max_ts, val_range)
            .expect("build chart");

        chart
            .configure_mesh()
            .x_label_formatter(&|v| format!("{:.1}s", v))
            .y_label_formatter(&|v| format!("{:.2}", v))
            .draw()
            .expect("draw mesh");

        // Halloy Ferra cyan #5dc1c5 for chart line
        let ferra_cyan = plotters::style::RGBColor(0x5d, 0xc1, 0xc5);
        chart
            .draw_series(LineSeries::new(self.data.iter().copied(), &ferra_cyan))
            .expect("draw series");
    }
}

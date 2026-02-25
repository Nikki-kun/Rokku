//! Rokku - ArduPilot log viewer with plotters-iced
//! Styled like Halloy IRC client (Ferra theme)

mod parser;

use iced::{
    event, window,
    widget::{
        button, column, mouse_area, row, text, tooltip, Column, Container, Scrollable, Space,
    },
    Alignment, Color as IcedColor, Element, Length, Point, Subscription, Task, Theme,
};
use plotters::prelude::*;
use plotters::style::Color;
use plotters_backend::DrawingBackend;
use plotters_iced::{Chart, ChartWidget};
use std::path::PathBuf;

use parser::{parse_log, LogRecord};

// Chart plot area: plotters margin 15, y_label 50, x_label 35 → plot from 65 to (width-15)
const CHART_PLOT_LEFT: f32 = 65.0;
const CHART_PLOT_RIGHT: f32 = 15.0;

// Halloy Ferra palette for charts
const FERRA_GRID: RGBColor = RGBColor(0x4f, 0x47, 0x4d);
const FERRA_LABEL: RGBColor = RGBColor(0xab, 0x8a, 0x79);
const CHART_COLORS: [RGBColor; 5] = [
    RGBColor(0x5d, 0xc1, 0xc5), // cyan
    RGBColor(0xf6, 0xb6, 0xc9), // pink
    RGBColor(0xb1, 0xb6, 0x95), // green
    RGBColor(0xff, 0xa0, 0x7a), // orange
    RGBColor(0xd7, 0xbd, 0xe2), // purple
];

fn ferra_container(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(0x24, 0x22, 0x26))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x4f, 0x47, 0x4d),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

/// Drop zone style: dashed border, subtle background
fn drop_zone_container(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(0x1e, 0x1c, 0x20))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x5d, 0xc1, 0xc5),
            width: 2.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

/// Modern chart card style
fn chart_card_container(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(0x1a, 0x18, 0x1c))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x3d, 0x3a, 0x40),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

/// Parameters panel style
fn params_panel_container(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(0x1e, 0x1c, 0x20))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x3d, 0x3a, 0x40),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

/// Format axis value: compact, no trailing zeros
fn format_axis_value(v: f64) -> String {
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn ferra_text(color: (u8, u8, u8)) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(IcedColor::from_rgb8(color.0, color.1, color.2)),
    }
}

/// Builds the floating tooltip content (table with param values at cursor time)
fn cursor_tooltip_content(app: &App) -> Element<'_, Message> {
    let Some(ref records) = app.records else {
        return text("").into();
    };
    let (min_ts, max_ts) = extract_time_range(records);
    let max_ts = max_ts.max(min_ts + 1.0);
    let cursor = app.cursor_time.unwrap_or(min_ts).clamp(min_ts, max_ts);

    if app.active_charts.is_empty() {
        return text("Добавьте графики").size(11).style(|_| ferra_text((0x68, 0x56, 0x50))).into();
    }

    let mut rows = Column::new().spacing(3);
    for param in &app.active_charts {
        let val_str = value_at_time(records, param, cursor)
            .map(|v| format!("{:.4}", v))
            .unwrap_or_else(|| "—".to_string());
        rows = rows.push(
            row![
                text(param.as_str())
                    .size(11)
                    .style(|_| ferra_text((0xfe, 0xcd, 0xb2))),
                Space::with_width(12),
                text(val_str)
                    .size(11)
                    .style(|_| ferra_text((0xb1, 0xb6, 0x95))),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }

    Container::new(
        column![
            text(format!("{:.2}s", cursor / 1000.0))
                .size(11)
                .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
            Space::with_height(4),
            rows,
        ]
        .spacing(2)
        .padding(10),
    )
    .style(ferra_container)
    .into()
}

fn time_slider_and_table(app: &App) -> Element<'_, Message> {
    let Some(ref records) = app.records else {
        return text("").into();
    };
    let (min_ts, max_ts) = extract_time_range(records);
    let max_ts = max_ts.max(min_ts + 1.0);
    let cursor = app.cursor_time.unwrap_or(min_ts).clamp(min_ts, max_ts);

    let time_display = row![
        text("Время курсора:")
            .size(12)
            .style(|_| ferra_text((0xab, 0x8a, 0x79))),
        text(format!("{:.2}s", cursor / 1000.0))
            .size(12)
            .style(|_| ferra_text((0xb1, 0xb6, 0x95))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    column![
        time_display,
        Space::with_height(4),
        text("Наведите курсор на график — таблица с данными следует за курсором")
            .size(11)
            .style(|_| ferra_text((0x68, 0x56, 0x50))),
    ]
    .spacing(4)
    .into()
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
    ParseComplete((PathBuf, Result<Vec<LogRecord>, String>)),
    AddChart(String),
    RemoveChart(usize),
    SetCursorTime(f64),
    WindowResized((iced::window::Id, iced::Size)),
}

struct App {
    file_path: String,
    records: Option<Vec<LogRecord>>,
    parameters: Vec<String>,
    active_charts: Vec<String>,
    cursor_time: Option<f64>,
    load_error: Option<String>,
    is_loading: bool,
    chart_width: f32,
}

impl App {
    fn new() -> (Self, Task<Message>) {
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

    fn subscription(_state: &App) -> Subscription<Message> {
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

    fn apply_parse_result(&mut self, path: PathBuf, result: Result<Vec<LogRecord>, String>) {
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
                // Chart area: window - params(280) - spacing(24) - padding(24*2)
                state.chart_width = (size.width - 328.0).max(400.0);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Loading state
        if self.is_loading {
            return Self::loading_view();
        }

        // Welcome screen: no file loaded
        if self.records.is_none() {
            return Self::welcome_view(self);
        }

        // Charts view: file loaded
        Self::charts_view(self)
    }

    fn welcome_view(app: &App) -> Element<'_, Message> {
        let drop_zone = mouse_area(
            Container::new(
                column![
                    text("📁")
                        .size(48)
                        .style(|_| ferra_text((0x5d, 0xc1, 0xc5))),
                    Space::with_height(12),
                    text("Перетащите .bin файл сюда")
                        .size(16)
                        .style(|_| ferra_text((0xfe, 0xcd, 0xb2))),
                    text("или нажмите для выбора из проводника")
                        .size(13)
                        .style(|_| ferra_text((0xab, 0x8a, 0x79))),
                ]
                .spacing(4)
                .align_x(Alignment::Center)
                .padding(40),
            )
            .width(Length::Fill)
            .center_x(Length::Fill)
            .style(drop_zone_container),
        )
        .on_press(Message::OpenFilePicker)
        .interaction(iced::mouse::Interaction::Pointer);

        let error_block: Element<Message> = if let Some(ref err) = app.load_error {
            column![
                text("Ошибка загрузки:")
                    .size(12)
                    .style(|_| ferra_text((0xe0, 0x6b, 0x75))),
                text(err.as_str())
                    .size(12)
                    .style(|_| ferra_text((0xfe, 0xcd, 0xb2))),
            ]
            .spacing(4)
            .into()
        } else {
            text("").into()
        };

        let welcome_content = column![
            Space::with_height(Length::FillPortion(2)),
            text("Rokku")
                .size(42)
                .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
            text("Просмотрщик логов ArduPilot")
                .size(16)
                .style(|_| ferra_text((0xab, 0x8a, 0x79))),
            Space::with_height(8),
            text("Загрузите Dataflash лог для анализа полёта")
                .size(13)
                .style(|_| ferra_text((0x68, 0x56, 0x50))),
            Space::with_height(8),
            error_block,
            Space::with_height(Length::FillPortion(2)),
            drop_zone,
            Space::with_height(30),
        ]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40);

        Container::new(welcome_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ferra_container)
            .into()
    }

    fn loading_view() -> Element<'static, Message> {
        let content = column![
            Space::with_height(Length::Fill),
            text("Rokku")
                .size(36)
                .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
            Space::with_height(24),
            text("Загрузка и парсинг лога...")
                .size(16)
                .style(|_| ferra_text((0x5d, 0xc1, 0xc5))),
            Space::with_height(8),
            text("Пожалуйста, подождите")
                .size(13)
                .style(|_| ferra_text((0xab, 0x8a, 0x79))),
            Space::with_height(Length::Fill),
        ]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(ferra_container)
            .into()
    }

    fn charts_view(&self) -> Element<'_, Message> {
        let controls = row![
            button("Открыть другой файл")
                .on_press(Message::OpenFilePicker)
                .style(button::secondary),
            Space::with_width(15),
            {
                let display_path = std::path::Path::new(&self.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&self.file_path);
                text(display_path)
                    .size(13)
                    .style(|_| ferra_text((0xb1, 0xb6, 0x95)))
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let content = column![
            controls,
            if let Some(ref err) = self.load_error {
                text(err).size(14).style(|_| ferra_text((0xe0, 0x6b, 0x75)))
            } else {
                text("")
            },
            Space::with_height(16),
        ];

        let params_list: Element<Message> = if self.parameters.is_empty() {
            text("Параметры появятся после загрузки лога")
                .size(14)
                .style(|_| ferra_text((0xab, 0x8a, 0x79)))
                .into()
        } else {
            let mut list = Column::new().spacing(2).padding(4);
            for p in &self.parameters {
                let param = p.clone();
                let can_add = !self.active_charts.contains(p);
                list = list.push(
                    row![
                        button("+")
                            .on_press_maybe(if can_add {
                                Some(Message::AddChart(param.clone()))
                            } else {
                                None
                            })
                            .style(if can_add {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .padding([4, 8]),
                        text(p.as_str())
                            .size(12)
                            .style(|_| ferra_text((0xfe, 0xcd, 0xb2))),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            Scrollable::new(list)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        let charts_section: Element<Message> = if let Some(ref records) = self.records {
            if self.active_charts.is_empty() {
                text("Нажмите + у параметра слева, чтобы добавить график")
                    .size(14)
                    .style(|_| ferra_text((0xab, 0x8a, 0x79)))
                    .into()
            } else {
                let (min_ts, max_ts) = extract_time_range(records);
                let max_ts = max_ts.max(min_ts + 1.0);
                let time_range = max_ts - min_ts;
                let mut charts_col = Column::new().spacing(16);
                for (idx, param) in self.active_charts.iter().enumerate() {
                    if let Some(data) = extract_param_series(records, param) {
                        let chart = LogChart::new(param.clone(), data, idx);
                        let i = idx;
                        let min_ts = min_ts;
                        let time_range = time_range;
                        let plot_width = self.chart_width - CHART_PLOT_LEFT - CHART_PLOT_RIGHT;
                        let chart_with_cursor = mouse_area(
                            ChartWidget::new(chart)
                                .width(Length::Fill)
                                .height(Length::Fixed(300.0)),
                        )
                        .on_move(move |p: Point| {
                            let frac = ((p.x - CHART_PLOT_LEFT) / plot_width).clamp(0.0, 1.0);
                            Message::SetCursorTime(min_ts + frac as f64 * time_range)
                        });
                        let chart_with_tooltip = tooltip(
                            chart_with_cursor,
                            cursor_tooltip_content(self),
                            iced::widget::tooltip::Position::FollowCursor,
                        )
                        .gap(8)
                        .padding(6)
                        .snap_within_viewport(true);
                        charts_col = charts_col.push(
                            Container::new(
                                column![
                                    row![
                                        text(param.as_str())
                                            .size(15)
                                            .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
                                        button("✕")
                                            .on_press(Message::RemoveChart(i))
                                            .style(button::danger)
                                            .padding([6, 12]),
                                    ]
                                    .spacing(12)
                                    .align_y(Alignment::Center),
                                    chart_with_tooltip,
                                ]
                                .spacing(12)
                                .padding(16),
                            )
                            .style(chart_card_container),
                        );
                    }
                }
                Scrollable::new(charts_col)
                    .height(Length::Fill)
                    .into()
            }
        } else {
            text("Загрузите ArduPilot Dataflash лог (.bin) для просмотра")
                .size(16)
                .style(|_| ferra_text((0xab, 0x8a, 0x79)))
                .into()
        };

        let charts_column = Container::new(
            column![
                text(format!("Графики ({})", self.active_charts.len()))
                    .size(17)
                    .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
                Space::with_height(12),
                time_slider_and_table(self),
                Space::with_height(12),
                charts_section,
            ]
            .width(Length::Fill)
            .spacing(8)
            .padding(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(chart_card_container);

        let data_panel = row![
            Container::new(
                column![
                    text("Параметры")
                        .size(17)
                        .style(|_| ferra_text((0xf5, 0xd7, 0x6e))),
                    text("+ добавить график")
                        .size(11)
                        .style(|_| ferra_text((0x68, 0x56, 0x50))),
                    Space::with_height(8),
                    params_list,
                ]
                .width(Length::Fill)
                .spacing(4)
                .padding(16),
            )
            .width(280)
            .style(params_panel_container),
            Space::with_width(24),
            charts_column,
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill);

        let main_content = column![content, data_panel]
            .spacing(24)
            .padding(24)
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

/// Returns (min_t, max_t) in same units as extract_param_series (ms)
fn extract_time_range(records: &[LogRecord]) -> (f64, f64) {
    if records.is_empty() {
        return (0.0, 1.0);
    }
    let (min_ns, max_ns) = records
        .iter()
        .map(|r| r.timestamp_ns as f64)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| (a.min(t), b.max(t)));
    (min_ns / 1_000_000.0, max_ns / 1_000_000.0)
}

/// Interpolated value at given time t (ms, same as series x) for a parameter
fn value_at_time(records: &[LogRecord], param: &str, t: f64) -> Option<f64> {
    let series = extract_param_series(records, param)?;
    if series.is_empty() {
        return None;
    }
    let first_ts = series[0].0;
    let last_ts = series.last()?.0;
    if t <= first_ts {
        return Some(series[0].1);
    }
    if t >= last_ts {
        return Some(series.last()?.1);
    }
    for i in 0..series.len() - 1 {
        let (t1, v1) = series[i];
        let (t2, v2) = series[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-9 {
                1.0
            } else {
                (t - t1) / (t2 - t1)
            };
            return Some(v1 + (v2 - v1) * frac);
        }
    }
    None
}

fn extract_parameters(records: &[LogRecord]) -> Vec<String> {
    let mut params: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for r in records {
        for field_name in r.fields.keys() {
            params.insert(format!("{}.{}", r.message_type, field_name));
        }
    }
    params.into_iter().collect()
}

/// Extract series for param "MessageType.FieldName"
fn extract_param_series(records: &[LogRecord], param: &str) -> Option<Vec<(f64, f64)>> {
    let mut parts = param.splitn(2, '.');
    let msg_type = parts.next()?;
    let field = parts.next()?;
    let mut points = Vec::new();
    for r in records {
        if r.message_type == msg_type {
            if let Some(v) = r.fields.get(field) {
                let val = v.as_f64().or_else(|| v.as_i64().map(|x| x as f64))?;
                points.push((r.timestamp_ns as f64 / 1_000_000.0, val));
            }
        }
    }
    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}

/// Chart for displaying log data with Ferra styling
struct LogChart {
    title: String,
    data: Vec<(f64, f64)>,
    color_idx: usize,
}

impl LogChart {
    fn new(title: String, data: Vec<(f64, f64)>, color_idx: usize) -> Self {
        Self {
            title,
            data,
            color_idx: color_idx % CHART_COLORS.len(),
        }
    }
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

        let val_margin = (max_val - min_val).abs().max(1.0) * 0.08;
        let val_range = (min_val - val_margin)..(max_val + val_margin);

        let mut chart = chart
            .caption(&self.title, ("sans-serif", 14).into_font().color(&FERRA_LABEL))
            .x_label_area_size(35)
            .y_label_area_size(50)
            .margin(15)
            .build_cartesian_2d(min_ts..max_ts, val_range)
            .expect("build chart");

        let label_style = ("sans-serif", 11).into_font().color(&FERRA_LABEL);
        chart
            .configure_mesh()
            .light_line_style(FERRA_GRID.mix(0.15))
            .bold_line_style(FERRA_GRID.mix(0.35))
            .axis_style(ShapeStyle::from(&FERRA_LABEL.mix(0.8)).stroke_width(1))
            .label_style(label_style)
            .x_label_formatter(&|v| format!("{:.1}s", v / 1000.0))
            .y_label_formatter(&|v| format_axis_value(*v))
            .x_labels(8)
            .y_labels(6)
            .draw()
            .expect("draw mesh");

        let line_color = CHART_COLORS[self.color_idx];
        let fill_color = line_color.mix(0.12);

        chart
            .draw_series(
                AreaSeries::new(
                    self.data.iter().map(|(x, y)| (*x, *y)),
                    min_val - val_margin,
                    fill_color,
                )
                .border_style(ShapeStyle::from(&line_color).stroke_width(2)),
            )
            .expect("draw area");

        chart
            .draw_series(LineSeries::new(
                self.data.iter().copied(),
                ShapeStyle::from(&line_color).stroke_width(2),
            ))
            .expect("draw line");
    }
}

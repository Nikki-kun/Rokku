use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use iced::{event, mouse, window, Subscription, Task};

use crate::config::{self, AppConfig};
use crate::data::{LogInfoEntry, ParamGroup, ParsedLog};
use crate::parser::parse_log;
pub use crate::table::{table_schema, TableColumnType, TableFileManagerTarget, TableId, TableRow};
use crate::ui::{
    charts_view, graph_builder_empty_view, loading_view, profile_view, settings_view, welcome_view,
};

const SIDEBAR_WIDTH: f32 = 50.0;
const SIDEBAR_GAP: f32 = 16.0;
const OUTER_PADDING: f32 = 12.0;
const MAIN_CONTENT_PADDING: f32 = 16.0;
const PARAMS_PANEL_WIDTH: f32 = 280.0;
const DATA_PANEL_SPACER: f32 = 24.0;
const DATA_PANEL_SPACING: f32 = 10.0;
const CHARTS_COLUMN_PADDING: f32 = 20.0;
const CHARTS_SCROLL_RIGHT_PADDING: f32 = 16.0;
const CHART_CARD_PADDING: f32 = 16.0;
const SCROLLBAR_COMPENSATION: f32 = 12.0;

fn compute_chart_widget_width(window_width: f32) -> f32 {
    let occupied_width = (OUTER_PADDING * 2.0)
        + SIDEBAR_WIDTH
        + SIDEBAR_GAP
        + (MAIN_CONTENT_PADDING * 2.0)
        + PARAMS_PANEL_WIDTH
        + DATA_PANEL_SPACER
        + (DATA_PANEL_SPACING * 2.0)
        + (CHARTS_COLUMN_PADDING * 2.0)
        + CHARTS_SCROLL_RIGHT_PADDING
        + (CHART_CARD_PADDING * 2.0)
        + SCROLLBAR_COMPENSATION;
    (window_width - occupied_width).max(100.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarItem {
    GraphBuilder,
    Table,
    Profile,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    FileDropped(PathBuf),
    OpenFilePicker,
    FileSelected(Option<PathBuf>),
    ParseComplete((PathBuf, Result<ParsedLog, String>)),
    AddChart(String),
    RemoveChart(usize),
    SetCursorTime(f64),
    WindowResized((iced::window::Id, iced::Size)),
    ParamSearchChanged(String),
    ParamGroupToggled(String),
    ChartZoom(String, mouse::ScrollDelta),
    ChartPanStart(String),
    ChartPanMove(String, f64, f32),
    ChartPanEnd,
    ToggleLogInfo,
    LogInfoSearchChanged(String),
    SidebarSelected(SidebarItem),
    SettingsPaletteChanged(String),
    SettingsServerAddressChanged(String),
    SettingsStrictTableFileBindingChanged(bool),
    SettingsMaxChartPointsChanged(String),
    SaveSettingsFinished(Result<(), String>),
    TableSelected(TableId),
    TableAddRow(TableId),
    TableRemoveRow(TableId, usize),
    TableCellChanged {
        table: TableId,
        row_index: usize,
        column: String,
        value: String,
    },
    TableOpenFileManager(TableId, usize),
    TableCloseFileManager,
    TableAddFiles(TableId, usize),
    TableFilesSelected(TableId, usize, Vec<PathBuf>),
    TableRemoveFile(TableId, usize, usize),
    TableOpenFileInGraph(TableId, usize, usize),
    SaveTableDataFinished(Result<(), String>),
    ProfileRegistrationNameChanged(String),
    ProfileRegistrationEmailChanged(String),
    ProfileRegistrationPasswordChanged(String),
    ProfileRegisterSubmitted,
    ProfileNameChanged(String),
    ProfileEmailChanged(String),
    ProfileOrganizationChanged(String),
    ProfileBioChanged(String),
    ProfileSaveChanges,
}

pub struct App {
    pub file_path: String,
    pub records: Option<ParsedLog>,
    pub parameters: Vec<String>,
    pub parameter_groups: Vec<ParamGroup>,
    pub expanded_groups: std::collections::HashSet<String>,
    pub param_search: String,
    pub active_charts: Vec<String>,
    pub chart_zoom: HashMap<String, (f64, f64)>,
    pub chart_pan_state: Option<(String, Option<f32>)>,
    pub cursor_time: Option<f64>,
    pub load_error: Option<String>,
    pub is_loading: bool,
    pub chart_width: f32,
    pub chart_bounds: Option<(f32, f32)>,
    pub active_sidebar: SidebarItem,
    pub show_welcome: bool,
    pub show_log_info: bool,
    pub log_info_entries: Vec<LogInfoEntry>,
    pub log_info_search: String,
    pub settings: AppConfig,
    pub settings_status: Option<String>,
    pub max_chart_points_input: String,
    pub active_table: TableId,
    pub table_rows: HashMap<TableId, Vec<TableRow>>,
    pub active_file_manager: Option<TableFileManagerTarget>,
    pub profile_registered: bool,
    pub profile_registration_name: String,
    pub profile_registration_email: String,
    pub profile_registration_password: String,
    pub profile_name: String,
    pub profile_email: String,
    pub profile_organization: String,
    pub profile_bio: String,
    pub profile_status: Option<String>,
}

impl App {
    fn table_data_path() -> PathBuf {
        if let Some(parent) = config::config_path().parent() {
            parent.join("tables.json")
        } else {
            PathBuf::from("tables.json")
        }
    }

    fn load_table_rows() -> HashMap<TableId, Vec<TableRow>> {
        let mut defaults = HashMap::from([
            (TableId::Flights, Vec::new()),
            (TableId::Missions, Vec::new()),
            (TableId::Drones, Vec::new()),
        ]);
        let path = Self::table_data_path();
        if !path.exists() {
            return defaults;
        }
        match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<HashMap<TableId, Vec<TableRow>>>(&raw) {
                Ok(mut parsed) => {
                    for (table, rows) in defaults.drain() {
                        parsed.entry(table).or_insert(rows);
                    }
                    parsed
                }
                Err(e) => {
                    log::warn!("Некорректный формат файла таблиц {}: {}", path.display(), e);
                    HashMap::from([
                        (TableId::Flights, Vec::new()),
                        (TableId::Missions, Vec::new()),
                        (TableId::Drones, Vec::new()),
                    ])
                }
            },
            Err(e) => {
                log::warn!("Не удалось прочитать файл таблиц {}: {}", path.display(), e);
                HashMap::from([
                    (TableId::Flights, Vec::new()),
                    (TableId::Missions, Vec::new()),
                    (TableId::Drones, Vec::new()),
                ])
            }
        }
    }

    fn save_table_data_task(table_rows: HashMap<TableId, Vec<TableRow>>) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let path = Self::table_data_path();
                    let Some(parent) = path.parent() else {
                        return Err(format!("Некорректный путь таблиц: {}", path.display()));
                    };
                    fs::create_dir_all(parent).map_err(|e| {
                        format!("Не удалось создать каталог {}: {}", parent.display(), e)
                    })?;
                    let raw = serde_json::to_string_pretty(&table_rows)
                        .map_err(|e| format!("Ошибка сериализации таблиц: {}", e))?;
                    fs::write(&path, raw).map_err(|e| {
                        format!("Не удалось сохранить таблицы {}: {}", path.display(), e)
                    })
                })
                .await
                .unwrap_or_else(|e| Err(format!("Ошибка сохранения таблиц: {}", e)))
            },
            Message::SaveTableDataFinished,
        )
    }

    pub fn table_reference_options(&self, table: TableId) -> Vec<String> {
        let Some(rows) = self.table_rows.get(&table) else {
            return Vec::new();
        };

        let mut options = Vec::with_capacity(rows.len());
        for (idx, row) in rows.iter().enumerate() {
            let label = match table {
                TableId::Missions => row
                    .cells
                    .get("missionObjective")
                    .filter(|v| !v.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("Миссия {}", idx + 1)),
                TableId::Drones => {
                    let factory = row
                        .cells
                        .get("factoryNumber")
                        .map(|v| v.trim())
                        .filter(|v| !v.is_empty())
                        .unwrap_or("");
                    let alternative = row
                        .cells
                        .get("alternativeNumber")
                        .map(|v| v.trim())
                        .filter(|v| !v.is_empty())
                        .unwrap_or("");
                    if !factory.is_empty() && !alternative.is_empty() {
                        format!("{} ({})", factory, alternative)
                    } else if !factory.is_empty() {
                        factory.to_string()
                    } else if !alternative.is_empty() {
                        alternative.to_string()
                    } else {
                        format!("Борт {}", idx + 1)
                    }
                }
                TableId::Flights => row
                    .cells
                    .get("mission")
                    .filter(|v| !v.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("Полет {}", idx + 1)),
            };
            options.push(label);
        }
        options
    }

    pub fn theme(&self) -> iced::Theme {
        match self.settings.palette.as_str() {
            "onedark" => iced::Theme::custom(
                "One Dark".to_string(),
                iced::theme::Palette {
                    background: iced::Color::from_rgb8(0x28, 0x2c, 0x34),
                    text: iced::Color::from_rgb8(0xab, 0xb2, 0xbf),
                    primary: iced::Color::from_rgb8(0x61, 0xaf, 0xef),
                    success: iced::Color::from_rgb8(0x98, 0xc3, 0x79),
                    danger: iced::Color::from_rgb8(0xe0, 0x6c, 0x75),
                },
            ),
            _ => iced::Theme::custom(
                "Gruvbox Dark".to_string(),
                iced::theme::Palette {
                    background: iced::Color::from_rgb8(0x28, 0x28, 0x28),
                    text: iced::Color::from_rgb8(0xeb, 0xdb, 0xb2),
                    primary: iced::Color::from_rgb8(0xd7, 0x99, 0x21),
                    success: iced::Color::from_rgb8(0x98, 0x97, 0x1a),
                    danger: iced::Color::from_rgb8(0xcc, 0x24, 0x1d),
                },
            ),
        }
    }

    fn save_settings_task(settings: AppConfig) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || config::save_config(&settings))
                    .await
                    .unwrap_or_else(|e| Err(format!("Ошибка сохранения задачи: {}", e)))
            },
            Message::SaveSettingsFinished,
        )
    }

    fn open_file_picker_task() -> Task<Message> {
        Task::perform(
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
        )
    }

    fn open_table_files_picker_task(table: TableId, row_index: usize) -> Task<Message> {
        Task::perform(
            async move {
                let files = tokio::task::spawn_blocking(|| {
                    rfd::FileDialog::new()
                        .set_title("Выберите файлы для прикрепления")
                        .pick_files()
                })
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
                (table, row_index, files)
            },
            |(table, row_index, files)| Message::TableFilesSelected(table, row_index, files),
        )
    }

    pub fn new() -> (Self, Task<Message>) {
        let (settings, settings_status) = match config::load_config() {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                log::warn!("{}", e);
                (
                    AppConfig::default(),
                    Some(format!(
                        "Конфиг не загружен, использованы значения по умолчанию: {}",
                        e
                    )),
                )
            }
        };
        let mut settings = settings;
        settings.palette = config::normalize_palette(&settings.palette);
        let max_chart_points_input = settings.max_chart_points.to_string();
        let table_rows = Self::load_table_rows();
        (
            Self {
                file_path: String::new(),
                records: None,
                parameters: Vec::new(),
                parameter_groups: Vec::new(),
                expanded_groups: std::collections::HashSet::new(),
                param_search: String::new(),
                active_charts: Vec::new(),
                chart_zoom: HashMap::new(),
                chart_pan_state: None,
                cursor_time: None,
                load_error: None,
                is_loading: false,
                chart_width: 1200.0,
                chart_bounds: None,
                active_sidebar: SidebarItem::GraphBuilder,
                show_welcome: true,
                show_log_info: false,
                log_info_entries: Vec::new(),
                log_info_search: String::new(),
                settings,
                settings_status,
                max_chart_points_input,
                active_table: TableId::Flights,
                table_rows,
                active_file_manager: None,
                profile_registered: false,
                profile_registration_name: String::new(),
                profile_registration_email: String::new(),
                profile_registration_password: String::new(),
                profile_name: String::new(),
                profile_email: String::new(),
                profile_organization: String::new(),
                profile_bio: String::new(),
                profile_status: None,
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

    pub fn apply_parse_result(&mut self, path: PathBuf, result: Result<ParsedLog, String>) {
        self.is_loading = false;
        match result {
            Ok(records) => {
                let n_params = records.parameters().len();
                log::info!("Parsed {} parameters from {}", n_params, path.display());
                self.file_path = path.to_string_lossy().to_string();
                self.parameters = records.parameters().to_vec();
                self.parameter_groups = records.parameter_groups().to_vec();
                self.records = Some(records);
                self.log_info_entries = self
                    .records
                    .as_ref()
                    .map(|r| r.info_entries().to_vec())
                    .unwrap_or_default();
                self.show_log_info = false;
                self.log_info_search.clear();
                self.load_error = None;
                self.active_charts.clear();
                self.chart_zoom.clear();
                self.chart_pan_state = None;
                self.expanded_groups.clear();
                let (min_ts, _) = self.records.as_ref().unwrap().time_range();
                self.cursor_time = Some(min_ts);
                if self.chart_bounds.is_none() {
                    let chart_widget_width = compute_chart_widget_width(self.chart_width);
                    self.chart_bounds = Some((0.0, chart_widget_width));
                }
            }
            Err(e) => {
                log::error!("Parse failed: {}", e);
                self.load_error = Some(e);
                self.records = None;
                self.parameters = Vec::new();
                self.parameter_groups = Vec::new();
                self.active_charts.clear();
                self.chart_zoom.clear();
                self.chart_pan_state = None;
                self.log_info_entries.clear();
                self.show_log_info = false;
                self.log_info_search.clear();
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
                state.show_welcome = false;
                state.active_sidebar = SidebarItem::GraphBuilder;
                state.is_loading = true;
                state.load_error = None;
                App::start_parse(path)
            }
            Message::OpenFilePicker => {
                state.show_welcome = false;
                state.active_sidebar = SidebarItem::GraphBuilder;
                App::open_file_picker_task()
            }
            Message::FileSelected(path) => {
                if let Some(p) = path {
                    state.show_welcome = false;
                    state.active_sidebar = SidebarItem::GraphBuilder;
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
                        let _ = records.validate_param_series(param.as_str());
                    }
                    log::debug!("Added chart: {}", param);
                    state.active_charts.push(param);
                }
                Task::none()
            }
            Message::RemoveChart(idx) => {
                if idx < state.active_charts.len() {
                    let param = state.active_charts[idx].clone();
                    state.active_charts.remove(idx);
                    state.chart_zoom.remove(&param);
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
                state.chart_width = size.width;
                let chart_widget_width = compute_chart_widget_width(size.width);
                state.chart_bounds = Some((0.0, chart_widget_width));
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
            Message::ChartZoom(param, delta) => {
                let delta_y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y,
                };
                if delta_y == 0.0 {
                    return Task::none();
                }
                let Some(ref records) = state.records else {
                    return Task::none();
                };
                let Some(data) = records.get_series(&param) else {
                    return Task::none();
                };
                let (data_min, data_max) = data
                    .iter()
                    .map(|(t, _)| *t)
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| {
                        (a.min(t), b.max(t))
                    });
                let full_range = (data_max - data_min).max(0.001);
                let pivot = state
                    .cursor_time
                    .unwrap_or(data_min)
                    .clamp(data_min, data_max);
                let (v_start, v_end) = state
                    .chart_zoom
                    .get(&param)
                    .copied()
                    .unwrap_or((data_min, data_max));
                let current_range = (v_end - v_start).max(0.001);
                let zoom_factor = if delta_y > 0.0 { 1.0 / 1.15 } else { 1.15 };
                let new_range = (current_range * zoom_factor).clamp(full_range * 0.01, full_range);
                let half = new_range / 2.0;
                let mut new_start = pivot - half;
                let mut new_end = pivot + half;
                if new_end - new_start > full_range {
                    new_start = data_min;
                    new_end = data_max;
                } else {
                    if new_start < data_min {
                        new_start = data_min;
                        new_end = (data_min + new_range).min(data_max);
                    }
                    if new_end > data_max {
                        new_end = data_max;
                        new_start = (data_max - new_range).max(data_min);
                    }
                }
                if (new_end - new_start) >= full_range * 0.99 {
                    state.chart_zoom.remove(&param);
                } else {
                    state.chart_zoom.insert(param, (new_start, new_end));
                }
                Task::none()
            }
            Message::ChartPanStart(param) => {
                if state.chart_zoom.contains_key(&param) {
                    state.chart_pan_state = Some((param, None));
                }
                Task::none()
            }
            Message::ChartPanMove(param, delta_time, current_x) => {
                if let Some((ref pan_param, _)) = state.chart_pan_state {
                    if pan_param != &param {
                        return Task::none();
                    }
                    state.chart_pan_state = Some((param.clone(), Some(current_x)));
                    let Some(ref records) = state.records else {
                        return Task::none();
                    };
                    let Some(data) = records.get_series(&param) else {
                        return Task::none();
                    };
                    let (data_min, data_max) = data
                        .iter()
                        .map(|(t, _)| *t)
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| {
                            (a.min(t), b.max(t))
                        });
                    let Some((v_start, v_end)) = state.chart_zoom.get(&param).copied() else {
                        return Task::none();
                    };
                    let current_range = (v_end - v_start).max(0.001);
                    let mut new_start = v_start - delta_time;
                    let mut new_end = v_end - delta_time;
                    if new_start < data_min {
                        new_start = data_min;
                        new_end = (data_min + current_range).min(data_max);
                    }
                    if new_end > data_max {
                        new_end = data_max;
                        new_start = (data_max - current_range).max(data_min);
                    }
                    state.chart_zoom.insert(param, (new_start, new_end));
                }
                Task::none()
            }
            Message::ChartPanEnd => {
                state.chart_pan_state = None;
                Task::none()
            }
            Message::ToggleLogInfo => {
                state.show_log_info = !state.show_log_info;
                if !state.show_log_info {
                    state.log_info_search.clear();
                }
                Task::none()
            }
            Message::LogInfoSearchChanged(value) => {
                state.log_info_search = value;
                Task::none()
            }
            Message::SidebarSelected(item) => {
                state.show_welcome = false;
                state.active_sidebar = item;
                Task::none()
            }
            Message::SettingsPaletteChanged(value) => {
                state.settings.palette = config::normalize_palette(&value);
                App::save_settings_task(state.settings.clone())
            }
            Message::SettingsServerAddressChanged(value) => {
                state.settings.server_address = value;
                App::save_settings_task(state.settings.clone())
            }
            Message::SettingsStrictTableFileBindingChanged(value) => {
                state.settings.strict_table_file_binding = value;
                App::save_settings_task(state.settings.clone())
            }
            Message::SettingsMaxChartPointsChanged(value) => {
                state.max_chart_points_input = value;
                let parsed_points = state
                    .max_chart_points_input
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|v| *v >= 10 && *v <= 100_000);
                if let Some(parsed_points) = parsed_points {
                    state.settings.max_chart_points = parsed_points;
                    state.settings_status = None;
                    App::save_settings_task(state.settings.clone())
                } else {
                    state.settings_status =
                        Some("MAX_CHART_POINTS должен быть числом от 10 до 100000".to_string());
                    Task::none()
                }
            }
            Message::SaveSettingsFinished(result) => {
                match result {
                    Ok(()) => {
                        let path = config::config_path();
                        state.settings_status =
                            Some(format!("Настройки сохранены: {}", path.display()));
                    }
                    Err(e) => {
                        state.settings_status = Some(e);
                    }
                }
                Task::none()
            }
            Message::TableSelected(table) => {
                state.active_table = table;
                Task::none()
            }
            Message::TableAddRow(table) => {
                let schema = table_schema(table);
                let mut row_cells = HashMap::new();
                for column in schema.columns {
                    row_cells.insert(column.name.to_string(), String::new());
                }
                state.table_rows.entry(table).or_default().push(TableRow {
                    cells: row_cells,
                    files: Vec::new(),
                });
                App::save_table_data_task(state.table_rows.clone())
            }
            Message::TableRemoveRow(table, row_index) => {
                if let Some(rows) = state.table_rows.get_mut(&table) {
                    if row_index < rows.len() {
                        rows.remove(row_index);
                        if let Some(active) = state.active_file_manager {
                            if active.table == table && active.row_index == row_index {
                                state.active_file_manager = None;
                            } else if active.table == table && active.row_index > row_index {
                                state.active_file_manager = Some(TableFileManagerTarget {
                                    table,
                                    row_index: active.row_index - 1,
                                });
                            }
                        }
                    }
                }
                App::save_table_data_task(state.table_rows.clone())
            }
            Message::TableCellChanged {
                table,
                row_index,
                column,
                value,
            } => {
                if let Some(rows) = state.table_rows.get_mut(&table) {
                    if let Some(row) = rows.get_mut(row_index) {
                        row.cells.insert(column, value);
                    }
                }
                App::save_table_data_task(state.table_rows.clone())
            }
            Message::TableOpenFileManager(table, row_index) => {
                state.active_file_manager = Some(TableFileManagerTarget { table, row_index });
                Task::none()
            }
            Message::TableCloseFileManager => {
                state.active_file_manager = None;
                Task::none()
            }
            Message::TableAddFiles(table, row_index) => {
                App::open_table_files_picker_task(table, row_index)
            }
            Message::TableFilesSelected(table, row_index, files) => {
                if let Some(rows) = state.table_rows.get_mut(&table) {
                    if let Some(row) = rows.get_mut(row_index) {
                        for path in files {
                            let path_string = path.to_string_lossy().to_string();
                            if !row.files.iter().any(|existing| existing == &path_string) {
                                row.files.push(path_string);
                            }
                        }
                    }
                }
                App::save_table_data_task(state.table_rows.clone())
            }
            Message::TableRemoveFile(table, row_index, file_index) => {
                if let Some(rows) = state.table_rows.get_mut(&table) {
                    if let Some(row) = rows.get_mut(row_index) {
                        if file_index < row.files.len() {
                            row.files.remove(file_index);
                        }
                    }
                }
                App::save_table_data_task(state.table_rows.clone())
            }
            Message::TableOpenFileInGraph(table, row_index, file_index) => {
                let Some(rows) = state.table_rows.get(&table) else {
                    return Task::none();
                };
                let Some(row) = rows.get(row_index) else {
                    return Task::none();
                };
                let Some(path_string) = row.files.get(file_index) else {
                    return Task::none();
                };
                let path = PathBuf::from(path_string);
                if !path.exists() {
                    state.load_error = Some(format!("Файл не найден: {}", path.display()));
                    return Task::none();
                }
                state.show_welcome = false;
                state.active_sidebar = SidebarItem::GraphBuilder;
                state.is_loading = true;
                state.load_error = None;
                App::start_parse(path)
            }
            Message::SaveTableDataFinished(result) => {
                if let Err(e) = result {
                    log::warn!("{}", e);
                }
                Task::none()
            }
            Message::ProfileRegistrationNameChanged(value) => {
                state.profile_registration_name = value;
                Task::none()
            }
            Message::ProfileRegistrationEmailChanged(value) => {
                state.profile_registration_email = value;
                Task::none()
            }
            Message::ProfileRegistrationPasswordChanged(value) => {
                state.profile_registration_password = value;
                Task::none()
            }
            Message::ProfileRegisterSubmitted => {
                let name = state.profile_registration_name.trim();
                let email = state.profile_registration_email.trim();
                let password = state.profile_registration_password.trim();
                if name.is_empty() || email.is_empty() || password.is_empty() {
                    state.profile_status = Some("Заполни имя, email и пароль".to_string());
                    return Task::none();
                }
                if !email.contains('@') {
                    state.profile_status = Some("Введите корректный email".to_string());
                    return Task::none();
                }
                state.profile_registered = true;
                state.profile_name = name.to_string();
                state.profile_email = email.to_string();
                state.profile_registration_password.clear();
                state.profile_organization = "Не указано".to_string();
                state.profile_bio = "Добавьте информацию о себе".to_string();
                state.profile_status = Some("Профиль создан".to_string());
                Task::none()
            }
            Message::ProfileNameChanged(value) => {
                state.profile_name = value;
                Task::none()
            }
            Message::ProfileEmailChanged(value) => {
                state.profile_email = value;
                Task::none()
            }
            Message::ProfileOrganizationChanged(value) => {
                state.profile_organization = value;
                Task::none()
            }
            Message::ProfileBioChanged(value) => {
                state.profile_bio = value;
                Task::none()
            }
            Message::ProfileSaveChanges => {
                if state.profile_name.trim().is_empty() || state.profile_email.trim().is_empty() {
                    state.profile_status = Some("Имя и email не могут быть пустыми".to_string());
                    return Task::none();
                }
                if !state.profile_email.contains('@') {
                    state.profile_status = Some("Введите корректный email".to_string());
                    return Task::none();
                }
                state.profile_status = Some("Изменения профиля сохранены".to_string());
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        if self.is_loading {
            return loading_view(self);
        }
        if self.show_welcome {
            return welcome_view(self);
        }
        match self.active_sidebar {
            SidebarItem::GraphBuilder => {
                if self.records.is_none() {
                    graph_builder_empty_view(self)
                } else {
                    charts_view(self)
                }
            }
            SidebarItem::Table => crate::ui::table_view(self),
            SidebarItem::Profile => profile_view(self),
            SidebarItem::Settings => settings_view(self),
        }
    }
}

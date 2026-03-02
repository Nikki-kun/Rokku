use iced::{
    widget::{
        button, checkbox, column, mouse_area, pick_list, row, scrollable, text, text_input,
        tooltip, Column, Container, Scrollable, Space,
    },
    Alignment, Element, Length, Point,
};
use iced_fonts::{nerd::Nerd, NERD_FONT};
use plotters_iced::ChartWidget;

use crate::app::{table_schema, App, Message, SidebarItem, TableColumnType, TableId};
use crate::chart::{LogChart, CHART_PLOT_LEFT, CHART_PLOT_RIGHT};
use crate::data::{pad_series_from_zero, LogInfoKind};
use crate::ui::{
    chart_card_container, chart_remove_button, nav_icon_button, param_add_button,
    param_add_disabled, param_icon_button, params_panel_container, themed_container, themed_text,
    ACCENT_COLOR, TEXT_DIM, TEXT_ERROR, TEXT_HIGHLIGHT, TEXT_INFO, TEXT_MUTED, TEXT_PRIMARY,
};

mod state_views;

pub use state_views::{graph_builder_empty_view, loading_view, welcome_view};

pub(super) fn persistent_sidebar(app: &App) -> Element<'_, Message> {
    let nav_button = |icon: Nerd, item: SidebarItem| {
        let is_active = app.active_sidebar == item;
        button(
            text(iced_fonts::nerd::icon_to_string(icon))
                .font(NERD_FONT)
                .size(20)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .style(move |theme| {
                    if is_active {
                        themed_text(theme, ACCENT_COLOR)
                    } else {
                        themed_text(theme, TEXT_MUTED)
                    }
                }),
        )
        .on_press(Message::SidebarSelected(item))
        .style(move |theme: &iced::Theme, status| {
            let mut style = nav_icon_button(theme, status);
            if is_active && status == button::Status::Active {
                style.background = None;
            }
            style
        })
        .padding([10, 10])
    };

    Container::new(
        column![
            nav_button(Nerd::ChartLine, SidebarItem::GraphBuilder),
            Space::with_height(10),
            nav_button(Nerd::Table, SidebarItem::Table),
            Space::with_height(Length::Fill),
            nav_button(Nerd::AccountCircle, SidebarItem::Profile),
            Space::with_height(10),
            nav_button(Nerd::Cog, SidebarItem::Settings),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([16, 8]),
    )
    .width(50)
    .height(Length::Fill)
    .style(params_panel_container)
    .into()
}

pub fn cursor_tooltip_content(app: &App) -> Element<'_, Message> {
    let Some(ref records) = app.records else {
        return text("").into();
    };
    let (min_ts, max_ts) = records.time_range();
    let max_ts = max_ts.max(min_ts + 1.0);
    let cursor = app.cursor_time.unwrap_or(min_ts).clamp(min_ts, max_ts);

    if app.active_charts.is_empty() {
        return text("Добавьте графики")
            .size(11)
            .style(|theme| themed_text(theme, TEXT_DIM))
            .into();
    }

    let mut rows = Column::new().spacing(3);
    for param in &app.active_charts {
        let val_str = records
            .value_at_time(param, cursor)
            .map(|v| format!("{:.4}", v))
            .unwrap_or_else(|| "—".to_string());
        rows = rows.push(
            row![
                text(param.as_str())
                    .size(11)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                Space::with_width(12),
                text(val_str)
                    .size(11)
                    .style(|theme| themed_text(theme, TEXT_INFO)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }

    Container::new(
        column![
            text(format!("{:.2}s", cursor / 1000.0))
                .size(11)
                .style(|theme| themed_text(theme, ACCENT_COLOR)),
            Space::with_height(4),
            rows,
        ]
        .spacing(2)
        .padding(10),
    )
    .style(themed_container)
    .into()
}

fn log_info_panel(app: &App) -> Element<'_, Message> {
    let mut params_col = Column::new().spacing(4);
    let mut meta_col = Column::new().spacing(4);
    let mut n_params = 0usize;
    let mut n_meta = 0usize;
    let search = app.log_info_search.trim().to_lowercase();
    let mut filtered_params = 0usize;
    let mut filtered_meta = 0usize;

    for entry in &app.log_info_entries {
        match entry.kind {
            LogInfoKind::Parameter => n_params += 1,
            LogInfoKind::Metadata => n_meta += 1,
        }

        let matches_search = search.is_empty()
            || entry.name.to_lowercase().contains(&search)
            || entry.value.to_lowercase().contains(&search);
        if !matches_search {
            continue;
        }

        let line = row![
            text(entry.name.as_str())
                .size(12)
                .width(Length::FillPortion(2))
                .style(|theme| themed_text(theme, TEXT_PRIMARY)),
            text(entry.value.as_str())
                .size(12)
                .width(Length::FillPortion(3))
                .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        match entry.kind {
            LogInfoKind::Parameter => {
                filtered_params += 1;
                params_col = params_col.push(line);
            }
            LogInfoKind::Metadata => {
                filtered_meta += 1;
                meta_col = meta_col.push(line);
            }
        }
    }

    if filtered_params == 0 {
        params_col = params_col.push(
            text("Ничего не найдено")
                .size(12)
                .style(|theme| themed_text(theme, TEXT_MUTED)),
        );
    }
    if filtered_meta == 0 {
        meta_col = meta_col.push(
            text("Ничего не найдено")
                .size(12)
                .style(|theme| themed_text(theme, TEXT_MUTED)),
        );
    }

    let search_row: Element<Message> = row![
        text_input("Поиск по имени или значению...", &app.log_info_search)
            .on_input(Message::LogInfoSearchChanged)
            .padding(8)
            .size(12)
            .width(Length::Fill),
        if app.log_info_search.is_empty() {
            Element::from(Space::with_width(0))
        } else {
            button(text(iced_fonts::nerd::icon_to_string(Nerd::X)).font(NERD_FONT))
                .on_press(Message::LogInfoSearchChanged(String::new()))
                .style(button::secondary)
                .padding([6, 8])
                .into()
        },
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into();

    let body = column![
        text(format!(
            "Информация лога: {} параметров, {} метаданных",
            filtered_params, filtered_meta
        ))
        .size(14)
        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        Space::with_height(8),
        search_row,
        Space::with_height(8),
        text(format!(
            "Параметры (показано {} из {})",
            filtered_params, n_params
        ))
        .size(13)
        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        Space::with_height(6),
        Scrollable::new(params_col)
            .height(Length::FillPortion(3))
            .width(Length::Fill),
        Space::with_height(10),
        Container::new(Space::with_height(1))
            .width(Length::Fill)
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(
                        palette.background.strong.color.scale_alpha(0.7),
                    )),
                    ..Default::default()
                }
            }),
        Space::with_height(10),
        Space::with_height(8),
        text(format!(
            "Метаданные (показано {} из {})",
            filtered_meta, n_meta
        ))
        .size(13)
        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        Space::with_height(6),
        Scrollable::new(meta_col)
            .height(Length::FillPortion(2))
            .width(Length::Fill),
    ]
    .spacing(2)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(12);

    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(chart_card_container)
        .into()
}

pub fn charts_view(app: &App) -> Element<'_, Message> {
    let display_path = std::path::Path::new(&app.file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&app.file_path);
    let controls = row![
        text(format!("Файл: {}", display_path))
            .size(13)
            .style(|theme| themed_text(theme, ACCENT_COLOR)),
        Space::with_width(12),
        button(
            row![
                text(iced_fonts::nerd::icon_to_string(Nerd::FolderOpen))
                    .font(NERD_FONT)
                    .size(12),
                text("Открыть другой лог").size(12),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenFilePicker)
        .style(param_icon_button)
        .padding([4, 8]),
        button(
            row![
                text(if app.show_log_info {
                    iced_fonts::nerd::icon_to_string(Nerd::ChartLine)
                } else {
                    "i".to_string()
                })
                .font(if app.show_log_info {
                    NERD_FONT
                } else {
                    iced::Font::DEFAULT
                })
                .size(12),
                text(if app.show_log_info {
                    "Открыть графики"
                } else {
                    "Открыть параметры"
                })
                .size(12),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleLogInfo)
        .style(param_icon_button)
        .padding([4, 8]),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = column![
        controls,
        if let Some(ref err) = app.load_error {
            text(err)
                .size(14)
                .style(|theme| themed_text(theme, TEXT_ERROR))
        } else {
            text("")
        },
        Space::with_height(16),
    ];

    let params_list: Element<Message> = if app.parameter_groups.is_empty() {
        text("Параметры появятся после загрузки лога")
            .size(14)
            .style(|theme| themed_text(theme, TEXT_MUTED))
            .into()
    } else {
        let search_lower = app.param_search.to_lowercase();
        let mut list = Column::new().spacing(1).padding(6);
        for group in &app.parameter_groups {
            let matches_search = search_lower.is_empty()
                || group.name.to_lowercase().contains(&search_lower)
                || group
                    .fields
                    .iter()
                    .any(|f| f.to_lowercase().contains(&search_lower));
            if !matches_search {
                continue;
            }
            let group_name = group.name.clone();
            let is_expanded = app.expanded_groups.contains(&group_name);
            let expand_icon = if is_expanded {
                Nerd::ChevronDown
            } else {
                Nerd::ChevronRight
            };
            list = list.push(
                row![
                    button(
                        text(iced_fonts::nerd::icon_to_string(expand_icon))
                            .font(NERD_FONT)
                            .size(12),
                    )
                    .on_press(Message::ParamGroupToggled(group_name.clone()))
                    .style(param_icon_button)
                    .padding([4, 6]),
                    text(&group.name)
                        .size(13)
                        .style(|theme| themed_text(theme, ACCENT_COLOR)),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .padding([2, 4]),
            );
            if is_expanded {
                for field in &group.fields {
                    let param = format!("{}.{}", group.name, field);
                    let matches_field_search =
                        search_lower.is_empty() || param.to_lowercase().contains(&search_lower);
                    if !matches_field_search {
                        continue;
                    }
                    let can_add = !app.active_charts.contains(&param);
                    let param_clone = param.clone();
                    list = list.push(
                        row![
                            Space::with_width(24),
                            button(
                                text(iced_fonts::nerd::icon_to_string(Nerd::Plus))
                                    .font(NERD_FONT)
                                    .size(12),
                            )
                            .on_press_maybe(if can_add {
                                Some(Message::AddChart(param_clone))
                            } else {
                                None
                            })
                            .style(if can_add {
                                param_add_button
                            } else {
                                param_add_disabled
                            })
                            .padding([4, 6]),
                            text(param.clone())
                                .size(12)
                                .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .padding([2, 4]),
                    );
                }
            }
        }
        Scrollable::new(list)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    };

    let charts_section: Element<Message> = if let Some(ref records) = app.records {
        if app.active_charts.is_empty() {
            text("Нажмите + у параметра слева, чтобы добавить график")
                .size(14)
                .style(|theme| themed_text(theme, TEXT_MUTED))
                .into()
        } else {
            let (min_ts, max_ts) = records.time_range();
            let max_ts = max_ts.max(min_ts + 1.0);
            let mut charts_col = Column::new().spacing(16);
            for (idx, param) in app.active_charts.iter().enumerate() {
                if let Some(data) = records.get_series(param) {
                    let chart_data = pad_series_from_zero(&data);
                    let chart_min_ts = chart_data.first().map(|(t, _)| *t).unwrap_or(min_ts);
                    let chart_max_ts = chart_data.last().map(|(t, _)| *t).unwrap_or(max_ts);
                    let _chart_time_range = (chart_max_ts - chart_min_ts).max(0.001);
                    let zoom_range = app.chart_zoom.get(param).copied();
                    let chart = LogChart::with_cursor(
                        String::new(),
                        chart_data,
                        idx,
                        app.cursor_time,
                        zoom_range,
                        app.settings.max_chart_points,
                    );
                    let param_clone = param.clone();
                    let param_for_scroll = param.clone();
                    let i = idx;
                    let chart_widget_width = if let Some((_, w)) = app.chart_bounds {
                        w.max(1.0)
                    } else {
                        (app.chart_width - 546.0).max(100.0)
                    };
                    let plot_width =
                        (chart_widget_width - CHART_PLOT_LEFT - CHART_PLOT_RIGHT).max(1.0);
                    let (zoom_start, zoom_end) = zoom_range.unwrap_or((chart_min_ts, chart_max_ts));
                    let zoomed_time_range = (zoom_end - zoom_start).max(0.001);
                    let chart_with_cursor = mouse_area(
                        ChartWidget::new(chart)
                            .width(Length::Fixed(chart_widget_width))
                            .height(Length::Fixed(300.0)),
                    )
                    .on_press(Message::ChartPanStart(param_clone.clone()))
                    .on_release(Message::ChartPanEnd)
                    .on_move(move |p: Point| {
                        let is_panning = matches!(
                            app.chart_pan_state,
                            Some((ref pan_param, _)) if pan_param == &param_clone
                        );
                        if is_panning {
                            let delta_time = match app.chart_pan_state {
                                Some((_, Some(last_x))) => {
                                    let delta_x = p.x - last_x;
                                    (delta_x as f64 / plot_width as f64) * zoomed_time_range
                                }
                                _ => 0.0,
                            };
                            Message::ChartPanMove(param_clone.clone(), delta_time, p.x)
                        } else {
                            let pos_in_plot = (p.x - CHART_PLOT_LEFT).max(0.0);
                            let frac = (pos_in_plot / plot_width).min(1.0) as f64;
                            Message::SetCursorTime(zoom_start + frac * zoomed_time_range)
                        }
                    })
                    .on_scroll(move |delta| Message::ChartZoom(param_for_scroll.clone(), delta));
                    let chart_with_tooltip = tooltip(
                        chart_with_cursor,
                        cursor_tooltip_content(app),
                        iced::widget::tooltip::Position::FollowCursor,
                    )
                    .gap(8)
                    .padding(6)
                    .snap_within_viewport(true);
                    charts_col = charts_col.push(
                        Container::new(
                            column![
                                row![
                                    Space::with_width(Length::Fill),
                                    button(
                                        text(iced_fonts::nerd::icon_to_string(Nerd::TrashCan))
                                            .font(NERD_FONT)
                                            .size(14),
                                    )
                                    .on_press(Message::RemoveChart(i))
                                    .style(chart_remove_button)
                                    .padding([6, 8]),
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
            Scrollable::new(
                Container::new(charts_col)
                    .width(Length::Fill)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 16.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
            )
            .height(Length::Fill)
            .into()
        }
    } else {
        text("Загрузите ArduPilot (.bin) или PX4 ULog (.ulg) для просмотра")
            .size(16)
            .style(|theme| themed_text(theme, TEXT_MUTED))
            .into()
    };

    let charts_column = Container::new(
        column![
            text(format!("Графики ({})", app.active_charts.len()))
                .size(17)
                .style(|theme| themed_text(theme, ACCENT_COLOR)),
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
                Space::with_height(8),
                row![
                    text_input("Поиск...", &app.param_search)
                        .on_input(Message::ParamSearchChanged)
                        .padding(8)
                        .size(12)
                        .width(Length::Fill),
                    if app.param_search.is_empty() {
                        Element::from(Space::with_width(0))
                    } else {
                        button(text(iced_fonts::nerd::icon_to_string(Nerd::X)).font(NERD_FONT))
                            .on_press(Message::ParamSearchChanged(String::new()))
                            .style(button::secondary)
                            .padding([6, 8])
                            .into()
                    },
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill),
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

    let main_content: Element<'_, Message> = if app.show_log_info {
        column![
            content,
            Container::new(log_info_panel(app))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(24)
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        column![content, data_panel]
            .spacing(24)
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    row![
        persistent_sidebar(app),
        Space::with_width(16),
        Container::new(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(themed_container)
    ]
    .spacing(0)
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn profile_view(app: &App) -> Element<'_, Message> {
    let flights_count = app
        .table_rows
        .get(&TableId::Flights)
        .map(Vec::len)
        .unwrap_or(0);
    let missions_count = app
        .table_rows
        .get(&TableId::Missions)
        .map(Vec::len)
        .unwrap_or(0);
    let drones_count = app
        .table_rows
        .get(&TableId::Drones)
        .map(Vec::len)
        .unwrap_or(0);
    let attached_files_count: usize = app
        .table_rows
        .values()
        .flat_map(|rows| rows.iter())
        .map(|row| row.files.len())
        .sum();

    let body: Element<Message> = if !app.profile_registered {
        Container::new(
            column![
                text("Регистрация")
                    .size(24)
                    .style(|theme| themed_text(theme, ACCENT_COLOR)),
                text("Создайте профиль для работы с таблицами и статистикой")
                    .size(13)
                    .style(|theme| themed_text(theme, TEXT_MUTED)),
                Space::with_height(12),
                text("Имя")
                    .size(12)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                text_input("Введите имя", &app.profile_registration_name)
                    .on_input(Message::ProfileRegistrationNameChanged)
                    .padding(8),
                Space::with_height(6),
                text("Email")
                    .size(12)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                text_input("name@example.com", &app.profile_registration_email)
                    .on_input(Message::ProfileRegistrationEmailChanged)
                    .padding(8),
                Space::with_height(6),
                text("Пароль")
                    .size(12)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                text_input("Минимум 6 символов", &app.profile_registration_password)
                    .on_input(Message::ProfileRegistrationPasswordChanged)
                    .padding(8),
                Space::with_height(10),
                button("Зарегистрироваться")
                    .on_press(Message::ProfileRegisterSubmitted)
                    .style(button::secondary),
                if let Some(status) = &app.profile_status {
                    text(status)
                        .size(12)
                        .style(|theme| themed_text(theme, TEXT_INFO))
                } else {
                    text("")
                }
            ]
            .spacing(4)
            .padding(20)
            .width(Length::Fill),
        )
        .style(chart_card_container)
        .width(Length::Fill)
        .into()
    } else {
        column![
            text("Профиль")
                .size(26)
                .style(|theme| themed_text(theme, ACCENT_COLOR)),
            Space::with_height(8),
            row![
                Container::new(
                    column![
                        text("Информация")
                            .size(15)
                            .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
                        Space::with_height(8),
                        text(format!("Имя: {}", app.profile_name))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        text(format!("Email: {}", app.profile_email))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        text(format!("Организация: {}", app.profile_organization))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    ]
                    .spacing(4)
                    .padding(16),
                )
                .style(chart_card_container)
                .width(Length::FillPortion(2)),
                Container::new(
                    column![
                        text("Статистика")
                            .size(15)
                            .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
                        Space::with_height(8),
                        text(format!("Полеты: {}", flights_count))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        text(format!("Миссии: {}", missions_count))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        text(format!("Борты: {}", drones_count))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                        text(format!("Файлы: {}", attached_files_count))
                            .size(13)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    ]
                    .spacing(4)
                    .padding(16),
                )
                .style(chart_card_container)
                .width(Length::FillPortion(2)),
            ]
            .spacing(12),
            Space::with_height(12),
            Container::new(
                column![
                    text("Редактирование профиля")
                        .size(15)
                        .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
                    Space::with_height(8),
                    text("Имя")
                        .size(12)
                        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    text_input("Имя", &app.profile_name)
                        .on_input(Message::ProfileNameChanged)
                        .padding(8),
                    Space::with_height(6),
                    text("Email")
                        .size(12)
                        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    text_input("Email", &app.profile_email)
                        .on_input(Message::ProfileEmailChanged)
                        .padding(8),
                    Space::with_height(6),
                    text("Организация")
                        .size(12)
                        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    text_input("Организация", &app.profile_organization)
                        .on_input(Message::ProfileOrganizationChanged)
                        .padding(8),
                    Space::with_height(6),
                    text("О себе")
                        .size(12)
                        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    text_input("Краткая информация", &app.profile_bio)
                        .on_input(Message::ProfileBioChanged)
                        .padding(8),
                    Space::with_height(10),
                    button("Сохранить изменения")
                        .on_press(Message::ProfileSaveChanges)
                        .style(button::secondary),
                    if let Some(status) = &app.profile_status {
                        text(status)
                            .size(12)
                            .style(|theme| themed_text(theme, TEXT_INFO))
                    } else {
                        text("")
                    }
                ]
                .spacing(4)
                .padding(16),
            )
            .style(chart_card_container)
            .width(Length::Fill),
        ]
        .spacing(0)
        .into()
    };

    let content = column![body]
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24);

    row![
        persistent_sidebar(app),
        Space::with_width(16),
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(themed_container)
    ]
    .spacing(0)
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn settings_view(app: &App) -> Element<'_, Message> {
    let content = column![
        text("Настройки")
            .size(26)
            .style(|theme| themed_text(theme, ACCENT_COLOR)),
        Space::with_height(16),
        text("Цветовая палитра")
            .size(13)
            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        pick_list(
            crate::config::AVAILABLE_PALETTES,
            Some(app.settings.palette.as_str()),
            |palette| Message::SettingsPaletteChanged(palette.to_string()),
        )
        .placeholder("Выберите палитру"),
        Space::with_height(10),
        text("Адрес сервера")
            .size(13)
            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        text_input("http://127.0.0.1:8080", &app.settings.server_address)
            .on_input(Message::SettingsServerAddressChanged)
            .padding(8),
        Space::with_height(10),
        checkbox(
            "Жесткая привязка файлов для таблицы",
            app.settings.strict_table_file_binding
        )
        .on_toggle(Message::SettingsStrictTableFileBindingChanged),
        Space::with_height(10),
        text("MAX_CHART_POINTS (10..100000)")
            .size(13)
            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        text_input("250", &app.max_chart_points_input)
            .on_input(Message::SettingsMaxChartPointsChanged)
            .padding(8),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24);

    row![
        persistent_sidebar(app),
        Space::with_width(16),
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(themed_container)
    ]
    .spacing(0)
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn table_view(app: &App) -> Element<'_, Message> {
    fn file_icon_for(path: &str) -> Nerd {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        match extension.as_deref() {
            Some("bin") | Some("ulg") => Nerd::ChartLine,
            Some("txt") | Some("md") | Some("json") | Some("toml") | Some("csv") => Nerd::FileCode,
            Some("png") | Some("jpg") | Some("jpeg") | Some("webp") => Nerd::FileImage,
            Some("pdf") => Nerd::FilePdfBox,
            _ => Nerd::File,
        }
    }

    fn file_size_label(path: &str) -> String {
        let Ok(metadata) = std::fs::metadata(path) else {
            return "—".to_string();
        };
        let size = metadata.len() as f64;
        if size >= 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} ГБ", size / 1024.0 / 1024.0 / 1024.0)
        } else if size >= 1024.0 * 1024.0 {
            format!("{:.2} МБ", size / 1024.0 / 1024.0)
        } else if size >= 1024.0 {
            format!("{:.2} КБ", size / 1024.0)
        } else {
            format!("{:.0} Б", size)
        }
    }

    fn is_log_file(path: &str) -> bool {
        std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("bin") || ext.eq_ignore_ascii_case("ulg"))
            .unwrap_or(false)
    }

    fn table_cell_width(column_type: TableColumnType) -> f32 {
        match column_type {
            TableColumnType::Number => 130.0,
            TableColumnType::Date => 140.0,
            TableColumnType::Select => 180.0,
            TableColumnType::Textarea => 260.0,
            TableColumnType::Text => 190.0,
        }
    }

    if let Some(target) = app.active_file_manager {
        let schema = table_schema(target.table);
        let files = app
            .table_rows
            .get(&target.table)
            .and_then(|rows| rows.get(target.row_index))
            .map(|row| row.files.as_slice())
            .unwrap_or(&[]);

        let mut files_rows = Column::new().spacing(10);
        if files.is_empty() {
            files_rows = files_rows.push(
                Container::new(
                    text("Файлы не добавлены")
                        .size(13)
                        .style(|theme| themed_text(theme, TEXT_MUTED)),
                )
                .style(chart_card_container)
                .padding(12),
            );
        } else {
            for (file_index, file_path) in files.iter().enumerate() {
                let file_name = std::path::Path::new(file_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(file_path.as_str());
                let icon = file_icon_for(file_path);
                let show_graph_action = is_log_file(file_path);
                let graph_action: Element<Message> = if show_graph_action {
                    let graph_tooltip = Container::new(
                        text("Открыть в графопостроителе")
                            .size(11)
                            .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                    )
                    .style(themed_container)
                    .padding([6, 8]);
                    tooltip(
                        button(
                            text(iced_fonts::nerd::icon_to_string(Nerd::ChartLine))
                                .font(NERD_FONT)
                                .size(14),
                        )
                        .on_press(Message::TableOpenFileInGraph(
                            target.table,
                            target.row_index,
                            file_index,
                        ))
                        .style(param_icon_button)
                        .padding([5, 8]),
                        graph_tooltip,
                        iced::widget::tooltip::Position::Top,
                    )
                    .gap(8)
                    .padding(6)
                    .into()
                } else {
                    Space::with_width(0).into()
                };

                files_rows = files_rows.push(
                    Container::new(
                        row![
                            text(iced_fonts::nerd::icon_to_string(icon))
                                .font(NERD_FONT)
                                .size(16)
                                .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
                            column![
                                text(file_name)
                                    .size(13)
                                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                                text(format!("{}  •  {}", file_size_label(file_path), file_path))
                                    .size(11)
                                    .style(|theme| themed_text(theme, TEXT_DIM)),
                            ]
                            .spacing(2)
                            .width(Length::Fill),
                            row![graph_action, {
                                let delete_tooltip = Container::new(
                                    text("Удалить путь")
                                        .size(11)
                                        .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                                )
                                .style(themed_container)
                                .padding([6, 8]);
                                tooltip(
                                    button(
                                        text(iced_fonts::nerd::icon_to_string(Nerd::TrashCan))
                                            .font(NERD_FONT)
                                            .size(13),
                                    )
                                    .on_press(Message::TableRemoveFile(
                                        target.table,
                                        target.row_index,
                                        file_index,
                                    ))
                                    .style(chart_remove_button)
                                    .padding([5, 8]),
                                    delete_tooltip,
                                    iced::widget::tooltip::Position::Top,
                                )
                                .gap(8)
                                .padding(6)
                            },]
                            .spacing(6)
                            .align_y(Alignment::Center)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .style(chart_card_container)
                    .padding(10),
                );
            }
        }

        let files_manager_content = column![
            row![
                text(format!(
                    "Файлы записи: {} #{}",
                    schema.display_name,
                    target.row_index + 1
                ))
                .size(22)
                .style(|theme| themed_text(theme, ACCENT_COLOR)),
                Space::with_width(Length::Fill),
                button("Назад к таблице")
                    .on_press(Message::TableCloseFileManager)
                    .style(param_icon_button)
                    .padding([6, 10]),
            ]
            .align_y(Alignment::Center),
            Space::with_height(6),
            row![button("Добавить файлы")
                .on_press(Message::TableAddFiles(target.table, target.row_index))
                .style(button::secondary),]
            .align_y(Alignment::Center),
            Space::with_height(12),
            Container::new(Scrollable::new(files_rows).height(Length::Fill),)
                .style(chart_card_container)
                .padding(12)
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24);

        return row![
            persistent_sidebar(app),
            Space::with_width(16),
            Container::new(files_manager_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(themed_container)
        ]
        .spacing(0)
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let active_schema = table_schema(app.active_table);
    let active_rows = app
        .table_rows
        .get(&app.active_table)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut table_switcher = row![].spacing(8).align_y(Alignment::Center);
    for table in [TableId::Flights, TableId::Missions, TableId::Drones] {
        let schema = table_schema(table);
        let is_active = table == app.active_table;
        table_switcher = table_switcher.push(
            button(text(schema.display_name).size(12).style(move |theme| {
                if is_active {
                    themed_text(theme, ACCENT_COLOR)
                } else {
                    themed_text(theme, TEXT_PRIMARY)
                }
            }))
            .on_press(Message::TableSelected(table))
            .style(param_icon_button)
            .padding([6, 10]),
        );
    }

    let mut header_row = row![Container::new(
        text("#")
            .size(12)
            .style(|theme| themed_text(theme, ACCENT_COLOR))
    )
    .width(54)]
    .spacing(8)
    .align_y(Alignment::Center);

    for column in active_schema.columns {
        let label = if column.required {
            format!("{} *", column.display_name)
        } else {
            column.display_name.to_string()
        };
        header_row = header_row.push(
            Container::new(
                text(label)
                    .size(12)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
            )
            .width(Length::Fixed(table_cell_width(column.column_type))),
        );
    }
    header_row = header_row.push(
        Container::new(
            text("Действие")
                .size(12)
                .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        )
        .width(Length::Fixed(150.0)),
    );
    header_row = header_row.push(
        Container::new(Space::with_width(1))
            .width(Length::Fixed(1.0))
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(
                        palette.background.strong.color.scale_alpha(0.7),
                    )),
                    ..Default::default()
                }
            }),
    );

    let mut table_rows = Column::new().spacing(8);
    if active_rows.is_empty() {
        table_rows = table_rows.push(
            Container::new(
                text("Таблица пустая. Нажми \"Добавить запись\".")
                    .size(13)
                    .style(|theme| themed_text(theme, TEXT_MUTED)),
            )
            .padding([10, 0]),
        );
    } else {
        for (row_index, row_data) in active_rows.iter().enumerate() {
            let mut data_row = row![Container::new(
                text((row_index + 1).to_string())
                    .size(12)
                    .style(|theme| themed_text(theme, TEXT_MUTED))
            )
            .width(Length::Fixed(54.0))]
            .spacing(8)
            .align_y(Alignment::Center);

            for column in active_schema.columns {
                let value = row_data.cells.get(column.name).cloned().unwrap_or_default();
                let cell_control: Element<Message> = match column.column_type {
                    TableColumnType::Select => {
                        let options = if !column.options.is_empty() {
                            column.options.iter().map(|s| (*s).to_string()).collect()
                        } else if let Some(reference_table) = column.reference {
                            app.table_reference_options(reference_table)
                        } else {
                            Vec::new()
                        };
                        let selected = if value.is_empty() { None } else { Some(value) };
                        pick_list(options, selected, {
                            let column_name = column.name.to_string();
                            move |selected_value| Message::TableCellChanged {
                                table: app.active_table,
                                row_index,
                                column: column_name.clone(),
                                value: selected_value,
                            }
                        })
                        .placeholder("Выбрать")
                        .width(Length::Fill)
                        .into()
                    }
                    TableColumnType::Date => text_input("YYYY-MM-DD", &value)
                        .on_input({
                            let column_name = column.name.to_string();
                            move |new_value| Message::TableCellChanged {
                                table: app.active_table,
                                row_index,
                                column: column_name.clone(),
                                value: new_value,
                            }
                        })
                        .padding(8)
                        .width(Length::Fill)
                        .into(),
                    TableColumnType::Number => text_input("0", &value)
                        .on_input({
                            let column_name = column.name.to_string();
                            move |new_value| Message::TableCellChanged {
                                table: app.active_table,
                                row_index,
                                column: column_name.clone(),
                                value: new_value,
                            }
                        })
                        .padding(8)
                        .width(Length::Fill)
                        .into(),
                    TableColumnType::Text | TableColumnType::Textarea => text_input("", &value)
                        .on_input({
                            let column_name = column.name.to_string();
                            move |new_value| Message::TableCellChanged {
                                table: app.active_table,
                                row_index,
                                column: column_name.clone(),
                                value: new_value,
                            }
                        })
                        .padding(8)
                        .width(Length::Fill)
                        .into(),
                };

                data_row = data_row.push(
                    Container::new(cell_control)
                        .width(Length::Fixed(table_cell_width(column.column_type))),
                );
            }

            data_row = data_row.push(
                Container::new(
                    row![
                        button(
                            text(iced_fonts::nerd::icon_to_string(Nerd::FolderOpen))
                                .font(NERD_FONT)
                                .size(13),
                        )
                        .on_press(Message::TableOpenFileManager(app.active_table, row_index))
                        .style(param_icon_button)
                        .padding([6, 8]),
                        button(
                            text(iced_fonts::nerd::icon_to_string(Nerd::TrashCan))
                                .font(NERD_FONT)
                                .size(13),
                        )
                        .on_press(Message::TableRemoveRow(app.active_table, row_index))
                        .style(chart_remove_button)
                        .padding([6, 8]),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fixed(150.0)),
            );
            data_row = data_row.push(
                Container::new(Space::with_width(1))
                    .width(Length::Fixed(1.0))
                    .style(|theme: &iced::Theme| {
                        let palette = theme.extended_palette();
                        iced::widget::container::Style {
                            background: Some(iced::Background::Color(
                                palette.background.strong.color.scale_alpha(0.7),
                            )),
                            ..Default::default()
                        }
                    }),
            );

            table_rows = table_rows.push(data_row);
        }
    }

    let table_content = Container::new(
        column![
            Container::new(header_row).padding(10),
            Container::new(Space::with_height(1))
                .width(Length::Fill)
                .style(|theme: &iced::Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(
                            palette.background.strong.color.scale_alpha(0.7),
                        )),
                        ..Default::default()
                    }
                }),
            Container::new(table_rows).padding(10),
        ]
        .spacing(0)
        .width(Length::Shrink),
    )
    .style(chart_card_container);

    let content = column![
        text("Таблицы")
            .size(24)
            .style(|theme| themed_text(theme, ACCENT_COLOR)),
        text(format!(
            "{}: {} записей",
            active_schema.display_name,
            active_rows.len()
        ))
        .size(13)
        .style(|theme| themed_text(theme, TEXT_MUTED)),
        Space::with_height(8),
        row![
            table_switcher,
            Space::with_width(Length::Fill),
            button("Добавить запись")
                .on_press(Message::TableAddRow(active_schema.id))
                .style(button::secondary),
        ]
        .align_y(Alignment::Center),
        Space::with_height(12),
        Scrollable::new(table_content)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .height(Length::Fill)
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24);

    row![
        persistent_sidebar(app),
        Space::with_width(16),
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(themed_container)
    ]
    .spacing(0)
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

use iced::{
    widget::{
        button, column, mouse_area, row, text, text_input, tooltip, Column, Container, Scrollable,
        Space,
    },
    Alignment, Element, Length, Point,
};
use iced_fonts::{nerd::to_text, nerd::Nerd, NERD_FONT};
use plotters_iced::ChartWidget;

use crate::app::{App, Message};
use crate::chart::{LogChart, CHART_PLOT_LEFT, CHART_PLOT_RIGHT};
use crate::data::{extract_param_series, extract_time_range, pad_series_from_zero, value_at_time};
use crate::ui::{
    chart_card_container, chart_remove_button, drop_zone_container, ferra_container, ferra_text,
    param_add_button, param_add_disabled, param_icon_button, params_panel_container, FERRA_ACCENT,
};

pub fn cursor_tooltip_content(app: &App) -> Element<'_, Message> {
    let Some(ref records) = app.records else {
        return text("").into();
    };
    let (min_ts, max_ts) = extract_time_range(records);
    let max_ts = max_ts.max(min_ts + 1.0);
    let cursor = app.cursor_time.unwrap_or(min_ts).clamp(min_ts, max_ts);

    if app.active_charts.is_empty() {
        return text("Добавьте графики")
            .size(11)
            .style(|_| ferra_text((0x68, 0x56, 0x50)))
            .into();
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
                .style(|_| ferra_text(FERRA_ACCENT)),
            Space::with_height(4),
            rows,
        ]
        .spacing(2)
        .padding(10),
    )
    .style(ferra_container)
    .into()
}

pub fn welcome_view(app: &App) -> Element<'_, Message> {
    let drop_zone = mouse_area(
        Container::new(
            column![
                to_text(Nerd::Folder)
                    .size(48)
                    .style(|_| ferra_text((0x5d, 0xc1, 0xc5))),
                Space::with_height(12),
                text("Перетащите .bin или .ulg файл сюда")
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
        text("Rokku").size(42).style(|_| ferra_text(FERRA_ACCENT)),
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

pub fn loading_view() -> Element<'static, Message> {
    let content = column![
        Space::with_height(Length::Fill),
        text("Rokku").size(36).style(|_| ferra_text(FERRA_ACCENT)),
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

pub fn charts_view(app: &App) -> Element<'_, Message> {
    let controls = row![
        button("Открыть другой файл")
            .on_press(Message::OpenFilePicker)
            .style(button::secondary),
        Space::with_width(15),
        {
            let display_path = std::path::Path::new(&app.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&app.file_path);
            text(display_path)
                .size(13)
                .style(|_| ferra_text((0xb1, 0xb6, 0x95)))
        },
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = column![
        controls,
        if let Some(ref err) = app.load_error {
            text(err).size(14).style(|_| ferra_text((0xe0, 0x6b, 0x75)))
        } else {
            text("")
        },
        Space::with_height(16),
    ];

    let params_list: Element<Message> = if app.parameter_groups.is_empty() {
        text("Параметры появятся после загрузки лога")
            .size(14)
            .style(|_| ferra_text((0xab, 0x8a, 0x79)))
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
                        .style(|_| ferra_text(FERRA_ACCENT)),
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
                                .style(|_| ferra_text((0xfe, 0xcd, 0xb2))),
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

    let charts_section: Element<Message> =
        if let Some(ref records) = app.records {
            if app.active_charts.is_empty() {
                text("Нажмите + у параметра слева, чтобы добавить график")
                    .size(14)
                    .style(|_| ferra_text((0xab, 0x8a, 0x79)))
                    .into()
            } else {
                let (min_ts, max_ts) = extract_time_range(records);
                let max_ts = max_ts.max(min_ts + 1.0);
                let mut charts_col = Column::new().spacing(16);
                for (idx, param) in app.active_charts.iter().enumerate() {
                    if let Some(data) = extract_param_series(records, param) {
                        let chart_data = pad_series_from_zero(&data);
                        let chart_min_ts = chart_data.first().map(|(t, _)| *t).unwrap_or(min_ts);
                        let chart_max_ts = chart_data.last().map(|(t, _)| *t).unwrap_or(max_ts);
                        let chart_time_range = (chart_max_ts - chart_min_ts).max(0.001);
                        let chart =
                            LogChart::with_cursor(String::new(), chart_data, idx, app.cursor_time);
                        let i = idx;
                        let chart_widget_width = if let Some((_, w)) = app.chart_bounds {
                            (w + 12.0).max(1.0)
                        } else {
                            (app.chart_width - 104.0 + 12.0).max(100.0)
                        };
                        let plot_width =
                            (chart_widget_width - CHART_PLOT_LEFT - CHART_PLOT_RIGHT).max(1.0);
                        let chart_with_cursor = mouse_area(
                            ChartWidget::new(chart)
                                .width(Length::Fill)
                                .height(Length::Fixed(300.0)),
                        )
                        .on_move(move |p: Point| {
                            let pos_in_plot = (p.x - CHART_PLOT_LEFT).max(0.0);
                            let frac = (pos_in_plot / plot_width).min(1.0) as f64;
                            Message::SetCursorTime(chart_min_ts + frac * chart_time_range)
                        });
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
                                        text(param.as_str())
                                            .size(15)
                                            .style(|_| ferra_text(FERRA_ACCENT)),
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
                Scrollable::new(Container::new(charts_col).width(Length::Fill).padding(
                    iced::Padding {
                        top: 0.0,
                        right: 16.0,
                        bottom: 0.0,
                        left: 0.0,
                    },
                ))
                .height(Length::Fill)
                .into()
            }
        } else {
            text("Загрузите ArduPilot (.bin) или PX4 ULog (.ulg) для просмотра")
                .size(16)
                .style(|_| ferra_text((0xab, 0x8a, 0x79)))
                .into()
        };

    let charts_column = Container::new(
        column![
            text(format!("Графики ({})", app.active_charts.len()))
                .size(17)
                .style(|_| ferra_text(FERRA_ACCENT)),
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
                    .style(|_| ferra_text(FERRA_ACCENT)),
                text("+ добавить график")
                    .size(11)
                    .style(|_| ferra_text((0x68, 0x56, 0x50))),
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

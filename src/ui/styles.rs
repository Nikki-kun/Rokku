use iced::widget::{button, container};
use iced::{Color as IcedColor, Theme};

pub fn ferra_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(
            0x24, 0x22, 0x26,
        ))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x4f, 0x47, 0x4d),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn drop_zone_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(
            0x1e, 0x1c, 0x20,
        ))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x5d, 0xc1, 0xc5),
            width: 2.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub fn chart_card_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(
            0x1a, 0x18, 0x1c,
        ))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x3d, 0x3a, 0x40),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub fn params_panel_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(IcedColor::from_rgb8(
            0x1e, 0x1c, 0x20,
        ))),
        border: iced::Border {
            color: IcedColor::from_rgb8(0x3d, 0x3a, 0x40),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub fn ferra_text(color: (u8, u8, u8)) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(IcedColor::from_rgb8(color.0, color.1, color.2)),
    }
}

/// Мягкий акцентный жёлтый (вместо горчичного)
pub const FERRA_ACCENT: (u8, u8, u8) = (0xe8, 0xdc, 0xa8);

/// Кнопка-иконка без рамки (для expand/collapse в панели параметров)
pub fn param_icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = IcedColor::from_rgb8(0xab, 0x8a, 0x79);
    let (background, text_color) = match status {
        button::Status::Hovered => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x2a, 0x28, 0x2c))),
            IcedColor::from_rgb8(0xe8, 0xdc, 0xa8),
        ),
        button::Status::Pressed => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x32, 0x30, 0x34))),
            IcedColor::from_rgb8(0xe8, 0xdc, 0xa8),
        ),
        _ => (None, text_color),
    };
    button::Style {
        background,
        text_color,
        border: iced::Border {
            color: IcedColor::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

/// Кнопка "+" без рамки (добавить график)
pub fn param_add_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = IcedColor::from_rgb8(0x5d, 0xc1, 0xc5);
    let (background, text_color) = match status {
        button::Status::Hovered => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x2a, 0x38, 0x3a))),
            IcedColor::from_rgb8(0x7d, 0xe1, 0xe5),
        ),
        button::Status::Pressed => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x32, 0x40, 0x42))),
            IcedColor::from_rgb8(0x7d, 0xe1, 0xe5),
        ),
        _ => (None, text_color),
    };
    button::Style {
        background,
        text_color,
        border: iced::Border {
            color: IcedColor::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

/// Кнопка удаления графика — красная иконка без рамки
pub fn chart_remove_button(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = IcedColor::from_rgb8(0xe0, 0x6b, 0x75);
    let (background, text_color) = match status {
        button::Status::Hovered => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x3a, 0x28, 0x2a))),
            IcedColor::from_rgb8(0xff, 0x8b, 0x95),
        ),
        button::Status::Pressed => (
            Some(iced::Background::Color(IcedColor::from_rgb8(0x42, 0x30, 0x32))),
            IcedColor::from_rgb8(0xff, 0x8b, 0x95),
        ),
        _ => (None, text_color),
    };
    button::Style {
        background,
        text_color,
        border: iced::Border {
            color: IcedColor::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

/// Кнопка "+" неактивная (параметр уже добавлен)
pub fn param_add_disabled(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: IcedColor::from_rgb8(0x68, 0x56, 0x50),
        border: iced::Border {
            color: IcedColor::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

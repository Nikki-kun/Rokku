//! UI styles (Ferra theme)

use iced::widget::container;
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

/// Drop zone style: dashed border, subtle background
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

/// Modern chart card style
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

/// Parameters panel style
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

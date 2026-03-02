use iced::widget::{button, container};
use iced::{Color as IcedColor, Theme};

// Gruvbox dark: bright_yellow
pub const ACCENT_COLOR: (u8, u8, u8) = (0xfa, 0xbd, 0x2f);
// Gruvbox dark: gray_245
pub const TEXT_MUTED: (u8, u8, u8) = (0x92, 0x83, 0x74);
// Gruvbox dark: dark4
pub const TEXT_DIM: (u8, u8, u8) = (0x7c, 0x6f, 0x64);
// Gruvbox dark: light1
pub const TEXT_PRIMARY: (u8, u8, u8) = (0xeb, 0xdb, 0xb2);
// Gruvbox dark: bright_green
pub const TEXT_INFO: (u8, u8, u8) = (0xb8, 0xbb, 0x26);
// Gruvbox dark: bright_blue
pub const TEXT_HIGHLIGHT: (u8, u8, u8) = (0x83, 0xa5, 0x98);
// Gruvbox dark: bright_red
pub const TEXT_ERROR: (u8, u8, u8) = (0xfb, 0x49, 0x34);

pub fn themed_container(_theme: &Theme) -> container::Style {
    let palette = _theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.background.base.color)),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn drop_zone_container(_theme: &Theme) -> container::Style {
    let palette = _theme.extended_palette();
    container::Style {
        background: None,
        border: iced::Border {
            color: palette.primary.base.color.scale_alpha(0.4),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub fn chart_card_container(_theme: &Theme) -> container::Style {
    let palette = _theme.extended_palette();
    container::Style {
        background: None,
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub fn params_panel_container(_theme: &Theme) -> container::Style {
    let palette = _theme.extended_palette();
    container::Style {
        background: None,
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

pub fn themed_text(theme: &Theme, color: (u8, u8, u8)) -> iced::widget::text::Style {
    let palette = theme.extended_palette();
    let mapped = match color {
        ACCENT_COLOR => palette.primary.strong.color,
        TEXT_MUTED => palette.background.base.text.scale_alpha(0.78),
        TEXT_DIM => palette.background.base.text.scale_alpha(0.58),
        TEXT_PRIMARY => palette.background.base.text,
        TEXT_INFO => palette.secondary.base.color,
        TEXT_HIGHLIGHT => palette.primary.base.color,
        TEXT_ERROR => palette.danger.base.color,
        _ => IcedColor::from_rgb8(color.0, color.1, color.2),
    };
    iced::widget::text::Style {
        color: Some(mapped),
    }
}

pub fn nav_icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = _theme.extended_palette();
    let text_color = palette.background.base.text.scale_alpha(0.6);
    let (border_color, text_color) = match status {
        button::Status::Hovered => (
            palette.background.strong.color,
            palette.background.base.text,
        ),
        button::Status::Pressed => (palette.primary.base.color, palette.primary.strong.color),
        _ => (IcedColor::TRANSPARENT, text_color),
    };
    button::Style {
        background: None,
        text_color,
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

pub fn param_icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = _theme.extended_palette();
    let text_color = palette.background.base.text.scale_alpha(0.7);
    let (border_color, text_color) = match status {
        button::Status::Hovered => (
            palette.background.strong.color,
            palette.background.base.text,
        ),
        button::Status::Pressed => (palette.primary.base.color, palette.primary.strong.color),
        _ => (IcedColor::TRANSPARENT, text_color),
    };
    button::Style {
        background: None,
        text_color,
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

pub fn param_add_button(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = _theme.extended_palette();
    let text_color = palette.primary.base.color;
    let (border_color, text_color) = match status {
        button::Status::Hovered => (
            palette.primary.base.color.scale_alpha(0.5),
            palette.primary.strong.color,
        ),
        button::Status::Pressed => (palette.primary.strong.color, palette.primary.strong.color),
        _ => (IcedColor::TRANSPARENT, text_color),
    };
    button::Style {
        background: None,
        text_color,
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

pub fn chart_remove_button(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = _theme.extended_palette();
    let text_color = palette.danger.base.color;
    let (border_color, text_color) = match status {
        button::Status::Hovered => (
            palette.danger.base.color.scale_alpha(0.5),
            palette.danger.strong.color,
        ),
        button::Status::Pressed => (palette.danger.strong.color, palette.danger.strong.color),
        _ => (IcedColor::TRANSPARENT, text_color),
    };
    button::Style {
        background: None,
        text_color,
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

pub fn param_add_disabled(_theme: &Theme, _status: button::Status) -> button::Style {
    let palette = _theme.extended_palette();
    button::Style {
        background: None,
        text_color: palette.background.strong.color,
        border: iced::Border {
            color: IcedColor::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
    }
}

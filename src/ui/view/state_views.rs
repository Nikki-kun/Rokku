use iced::{
    widget::{button, column, mouse_area, row, text, Container, Space},
    Alignment, Element, Length,
};
use iced_fonts::{nerd::to_text, nerd::Nerd};

use crate::app::{App, Message, SidebarItem};
use crate::ui::{
    drop_zone_container, themed_container, themed_text, ACCENT_COLOR, TEXT_DIM, TEXT_ERROR,
    TEXT_HIGHLIGHT, TEXT_MUTED, TEXT_PRIMARY,
};

use super::persistent_sidebar;

pub fn welcome_view(app: &App) -> Element<'_, Message> {
    let welcome_content = column![
        Space::with_height(Length::FillPortion(2)),
        text("Rokku")
            .size(42)
            .style(|theme| themed_text(theme, ACCENT_COLOR)),
        text("Просмотрщик логов ArduPilot")
            .size(16)
            .style(|theme| themed_text(theme, TEXT_MUTED)),
        Space::with_height(8),
        text("Загрузите Dataflash лог для анализа полёта")
            .size(13)
            .style(|theme| themed_text(theme, TEXT_DIM)),
        Space::with_height(20),
        button("Открыть графопостроитель")
            .on_press(Message::SidebarSelected(SidebarItem::GraphBuilder))
            .style(button::secondary),
        Space::with_height(Length::FillPortion(3)),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(40);

    row![
        persistent_sidebar(app),
        Space::with_width(16),
        Container::new(welcome_content)
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

pub fn graph_builder_empty_view(app: &App) -> Element<'_, Message> {
    let drop_zone = mouse_area(
        Container::new(
            column![
                to_text(Nerd::Folder)
                    .size(48)
                    .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
                Space::with_height(12),
                text("Перетащите .bin или .ulg файл сюда")
                    .size(16)
                    .style(|theme| themed_text(theme, TEXT_PRIMARY)),
                text("или нажмите для выбора файла")
                    .size(13)
                    .style(|theme| themed_text(theme, TEXT_MUTED)),
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
                .style(|theme| themed_text(theme, TEXT_ERROR)),
            text(err.as_str())
                .size(12)
                .style(|theme| themed_text(theme, TEXT_PRIMARY)),
        ]
        .spacing(4)
        .into()
    } else {
        text("").into()
    };

    let content = column![
        text("Графопостроитель")
            .size(24)
            .style(|theme| themed_text(theme, ACCENT_COLOR)),
        Space::with_height(8),
        error_block,
        Space::with_height(Length::Fill),
        drop_zone,
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

pub fn loading_view(app: &App) -> Element<'_, Message> {
    let content = column![
        Space::with_height(Length::Fill),
        text("Загрузка и парсинг лога...")
            .size(16)
            .style(|theme| themed_text(theme, TEXT_HIGHLIGHT)),
        Space::with_height(8),
        text("Пожалуйста, подождите")
            .size(13)
            .style(|theme| themed_text(theme, TEXT_MUTED)),
        Space::with_height(Length::Fill),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

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

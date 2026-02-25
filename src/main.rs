//! Rokku - ArduPilot log viewer with plotters-iced
//! Styled like Halloy IRC client (Ferra theme)

mod app;
mod chart;
mod data;
mod parser;
mod ui;

fn main() -> iced::Result {
    iced::application("Rokku - ArduPilot Log Viewer", app::App::update, app::App::view)
        .theme(|_| iced::Theme::Ferra)
        .subscription(app::App::subscription)
        .antialiasing(true)
        .run_with(app::App::new)
}

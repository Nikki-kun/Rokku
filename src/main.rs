#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod chart;
mod config;
mod data;
mod parser;
mod table;
mod ui;

fn main() -> iced::Result {
    #[cfg(any(debug_assertions, not(target_os = "windows")))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    #[cfg(any(debug_assertions, not(target_os = "windows")))]
    log::info!("Starting Rokku");

    iced::application(
        "Rokku - ArduPilot Log Viewer",
        app::App::update,
        app::App::view,
    )
    .font(iced_fonts::NERD_FONT_BYTES)
    .theme(app::App::theme)
    .subscription(app::App::subscription)
    .antialiasing(true)
    .run_with(app::App::new)
}

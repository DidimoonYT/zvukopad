//! Звукопад — аналог Soundpad на Rust.
//!
//! Точка входа. Скрывает консоль в release-сборке на Windows и запускает
//! графическое окно eframe.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod config;
mod hotkeys;
mod kb_capture;
mod ptt;

use app::ZvukopadApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();

    log::info!("Запуск Звукопада…");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Звукопад")
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Звукопад",
        options,
        Box::new(|_cc| Ok(Box::new(ZvukopadApp::new()))),
    )
}

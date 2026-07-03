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
mod version;

use app::ZvukopadApp;
use eframe::egui;
use std::sync::Arc;

fn load_icon() -> Arc<egui::IconData> {
    // Загружаем иконку из файлайненированных в бинарник байт (compile-time)
    let icon_bytes = include_bytes!("../icon.ico");
    let icon_image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon.ico")
        .into_rgba8();
    let (width, height) = icon_image.dimensions();
    Arc::new(egui::IconData {
        rgba: icon_image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .try_init();

    let app_name = version::version_display();

    log::info!("Запуск Звукопада {}…", version::VERSION);

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&app_name)
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([640.0, 400.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        &app_name,
        options,
        Box::new(|_cc| Ok(Box::new(ZvukopadApp::new()))),
    )
}

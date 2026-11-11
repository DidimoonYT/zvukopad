//! Тема оформления «спокойный тёмный Material 3».
//! Плоские цвета, скругления, воздух. Без градиентов, теней и размытий.

// Палитра и токены дизайн-системы описаны целиком, даже если часть
// из них пока не задействована в интерфейсе.
#![allow(dead_code)]

use eframe::egui;

/// Скругления (u8, т.к. CornerRadius хранит u8).
pub const CORNER_CARD: u8 = 20;
pub const CORNER_FIELD: u8 = 12;
pub const CORNER_PILL: u8 = 20;
pub const CORNER_SWITCH: u8 = 13;

/// Отступы.
pub const PAD_SCREEN: f32 = 16.0;
pub const PAD_CARD: f32 = 16.0;
pub const PAD_BETWEEN: f32 = 12.0;

/// Размеры шрифта.
pub const FONT_TITLE: f32 = 26.0;
pub const FONT_CARD_HEADER: f32 = 16.0;
pub const FONT_BODY: f32 = 14.0;
pub const FONT_CAPTION: f32 = 12.0;

fn hex_u32(hex: u32) -> egui::Color32 {
    egui::Color32::from_rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

/// ================= Тёмная тема =================
pub mod dark {
    use super::*;

    pub const BG: u32 = 0x141218;
    pub const CARD: u32 = 0x191923;
    pub const BLOCK: u32 = 0x212025;
    pub const TEXT: u32 = 0xE6E0E9;
    pub const SECONDARY: u32 = 0xCAC4D0;
    pub const MUTED: u32 = 0x938F99;
    pub const BORDER: u32 = 0x3A3842;
    pub const ACCENT: u32 = 0x7DA2FF;
    pub const ACCENT_BORDER: u32 = 0x1B2B52;
    pub const SUCCESS: u32 = 0x1BAA60;
    pub const WARNING: u32 = 0xE0A100;
    pub const ERROR: u32 = 0xE5484D;
    pub const WARNING_BG: u32 = 0xFFF7ED;
    pub const WARNING_BORDER: u32 = 0xFDBA74;
    pub const WARNING_TEXT: u32 = 0x8C3E09;

    pub fn bg() -> egui::Color32 { hex_u32(BG) }
    pub fn card() -> egui::Color32 { hex_u32(CARD) }
    pub fn block() -> egui::Color32 { hex_u32(BLOCK) }
    pub fn text() -> egui::Color32 { hex_u32(TEXT) }
    pub fn secondary() -> egui::Color32 { hex_u32(SECONDARY) }
    pub fn muted() -> egui::Color32 { hex_u32(MUTED) }
    pub fn border() -> egui::Color32 { hex_u32(BORDER) }
    pub fn accent() -> egui::Color32 { hex_u32(ACCENT) }
    pub fn accent_border() -> egui::Color32 { hex_u32(ACCENT_BORDER) }
    pub fn success() -> egui::Color32 { hex_u32(SUCCESS) }
    pub fn warning() -> egui::Color32 { hex_u32(WARNING) }
    pub fn error() -> egui::Color32 { hex_u32(ERROR) }
    pub fn warning_bg() -> egui::Color32 { hex_u32(WARNING_BG) }
    pub fn warning_border() -> egui::Color32 { hex_u32(WARNING_BORDER) }
    pub fn warning_text() -> egui::Color32 { hex_u32(WARNING_TEXT) }
    pub fn accent_text() -> egui::Color32 { hex_u32(0x141218) }
}

/// ================= Светлая тема =================
pub mod light {
    use super::*;

    pub const BG: u32 = 0xFFFBFE;
    pub const CARD: u32 = 0xF3EDF7;
    pub const TEXT: u32 = 0x1C1B1F;
    pub const ACCENT: u32 = 0x4F7CFF;

    pub fn bg() -> egui::Color32 { hex_u32(BG) }
    pub fn card() -> egui::Color32 { hex_u32(CARD) }
    pub fn text() -> egui::Color32 { hex_u32(TEXT) }
    pub fn accent() -> egui::Color32 { hex_u32(ACCENT) }
}

/// Применяет визуальный стиль egui к контексту.
pub fn apply_visuals(ctx: &egui::Context, dark: bool) {
    if dark {
        let mut v = egui::Visuals::dark();
        v.window_fill = dark::bg();
        v.panel_fill = dark::bg();
        v.widgets.noninteractive.bg_fill = dark::bg();
        v.widgets.inactive.bg_fill = dark::card();
        v.widgets.active.bg_fill = dark::accent();
        v.widgets.hovered.bg_fill = dark::block();
        v.widgets.open.bg_fill = dark::card();

        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, dark::text());
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, dark::secondary());
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, dark::accent_text());
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, dark::text());
        v.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, dark::text());

        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, dark::border());
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, dark::border());
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, dark::accent_border());
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, dark::accent());
        v.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, dark::border());

        v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(CORNER_CARD);
        v.widgets.inactive.corner_radius = egui::CornerRadius::same(CORNER_CARD);
        v.widgets.active.corner_radius = egui::CornerRadius::same(CORNER_PILL);
        v.widgets.hovered.corner_radius = egui::CornerRadius::same(CORNER_PILL);
        v.widgets.open.corner_radius = egui::CornerRadius::same(CORNER_CARD);

        v.selection.bg_fill = dark::accent();
        v.selection.stroke = egui::Stroke::NONE;
        v.hyperlink_color = dark::accent();
        v.faint_bg_color = dark::block();
        v.window_corner_radius = egui::CornerRadius::same(CORNER_CARD);
        v.window_stroke = egui::Stroke::new(1.0_f32, dark::border());
        v.window_shadow = egui::epaint::Shadow::NONE;

        ctx.set_visuals(v);
    } else {
        let mut v = egui::Visuals::light();
        v.window_fill = light::bg();
        v.panel_fill = light::bg();
        v.widgets.noninteractive.bg_fill = light::bg();
        v.widgets.inactive.bg_fill = light::card();
        v.widgets.active.bg_fill = light::accent();
        v.widgets.hovered.bg_fill = light::card();

        v.window_stroke = egui::Stroke::new(1.0_f32, light::accent());
        v.window_shadow = egui::epaint::Shadow::NONE;
        ctx.set_visuals(v);
    }
}

/// Карточка: скруглённый фон, обводка, внутренние и внешние отступы.
pub fn card_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(dark::card())
        .corner_radius(egui::CornerRadius::same(CORNER_CARD))
        .stroke(egui::Stroke::new(1.0_f32, dark::border()))
        .inner_margin(egui::vec2(PAD_CARD, PAD_CARD))
        .outer_margin(egui::vec2(0.0, PAD_BETWEEN))
}

/// Компактная карточка для панели настроек: меньше вертикальные поля,
/// чтобы весь столбец помещался в окно без прокрутки.
pub fn settings_card() -> egui::Frame {
    egui::Frame::NONE
        .fill(dark::card())
        .corner_radius(egui::CornerRadius::same(CORNER_CARD))
        .stroke(egui::Stroke::new(1.0_f32, dark::border()))
        .inner_margin(egui::vec2(PAD_CARD, 12.0))
        .outer_margin(egui::vec2(0.0, 6.0))
}

/// Блок внутри карточки (второй уровень вложенности).
pub fn block_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(dark::block())
        .corner_radius(egui::CornerRadius::same(CORNER_FIELD))
        .stroke(egui::Stroke::new(1.0_f32, dark::border()))
        .inner_margin(egui::vec2(PAD_CARD, PAD_CARD))
        .outer_margin(egui::vec2(0.0, PAD_BETWEEN))
}

/// Пилюльная акцентная кнопка с тёмным текстом.
pub fn pill_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let rich = egui::RichText::new(text)
        .color(dark::accent_text())
        .strong();
    ui.add(
        egui::Button::new(rich)
            .fill(dark::accent())
            .stroke(egui::Stroke::new(1.0_f32, dark::accent_border()))
            .corner_radius(egui::CornerRadius::same(CORNER_PILL))
    )
}

/// Вторичная кнопка (без фона, текст основным цветом).
pub fn text_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text).color(dark::text())
        )
            .fill(dark::block())
            .stroke(egui::Stroke::new(1.0_f32, dark::border()))
            .corner_radius(egui::CornerRadius::same(CORNER_PILL))
    )
}

/// Маленькая вторичная кнопка.
pub fn small_text_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text).size(FONT_CAPTION).color(dark::muted())
        )
            .fill(dark::block())
            .stroke(egui::Stroke::new(1.0_f32, dark::border()))
            .corner_radius(egui::CornerRadius::same(CORNER_FIELD))
    )
}

/// Точка статуса (цветной кружок) с текстом рядом.
pub fn status_row(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let size = 10.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::empty());
        let center = rect.center();
        ui.painter().circle_filled(center, size / 2.0, color);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(label)
                .size(FONT_BODY)
                .color(dark::text())
        );
    });
}

/// Заголовок карточки.
pub fn card_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(FONT_CARD_HEADER)
            .strong()
            .color(dark::text())
    );
}

/// Основной текст.
pub fn body(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(FONT_BODY)
            .color(dark::text())
    );
}

/// Второстепенный текст.
pub fn secondary_text(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(FONT_BODY)
            .color(dark::secondary())
    );
}

/// Подпись (приглушённый текст).
pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(FONT_CAPTION)
            .color(dark::muted())
    );
}

/// Заголовок приложения (имя, 26 bold).
pub fn app_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(FONT_TITLE)
            .strong()
            .color(dark::text())
    );
}

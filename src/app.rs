//! Графический интерфейс приложения на egui.
//!
//! Стиль: «спокойный тёмный Material 3» (см. src/theme.rs).
//! Тёмно-графитовый фон, крупные скруглённые карточки без теней,
//! один акцентный сине-перивинкловый цвет, пилюльные кнопки,
//! цветные точки статуса, много воздуха.

use crate::audio::{self, AudioEngine, DEFAULT_DEVICE_NAME, NO_DEVICE_NAME};
use crate::config::{Config, HotkeyConfig, SoundEntry};
use crate::hotkeys::{self, HotkeyAction, HotkeyManager};
use crate::theme::{
    self, app_title, body, caption, card_frame, card_title, dark, pill_button, secondary_text,
    settings_card, small_text_button, status_row, text_button, FONT_BODY, PAD_BETWEEN, PAD_CARD,
    PAD_SCREEN,
};
use eframe::egui;
use eframe::egui::CornerRadius;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// Состояние диалога добавления/редактирования записи.
struct PendingEdit {
    index: usize,
    name: String,
    path: String,
    volume: f32,
}

/// Состояние захвата горячей клавиши.
#[derive(Clone, Copy, PartialEq)]
enum CaptureTarget {
    /// Назначение клавиши на запись (индекс).
    Sound(usize),
    /// Глобальная клавиша «Остановить всё».
    StopAll,
    /// Клавиша PTT (имитация микрофона).
    Ptt,
}

/// Главное приложение.
pub struct ZvukopadApp {
    config: Config,
    audio: Rc<RefCell<AudioEngine>>,
    hotkeys: Rc<RefCell<HotkeyManager>>,
    devices: Vec<audio::DeviceInfo>,
    /// Индекс выбранного ОСНОВНОГО устройства.
    selected_main_device: usize,
    /// Индекс выбранного устройства МОНИТОРИНГА (0 = нет).
    selected_monitor_device: usize,
    status: String,
    edit: Option<PendingEdit>,
    capturing: Option<CaptureTarget>,
    dirty: bool,
    /// Момент последнего обновления списка устройств.
    last_device_refresh: Instant,
    /// Для debounce после захвата клавиши (чтобы не поймать отпускание).
    last_capture_time: Instant,
    /// Момент последней записи конфига (дебаунс автосохранения).
    last_save: Instant,
    /// Тема применяется один раз: `ctx.set_visuals` каждый кадр заставляет
    /// egui пересобирать стиль и лишний раз перерисовывать всё окно.
    theme_applied: bool,
}

impl ZvukopadApp {
    pub fn new() -> Self {
        let config = Config::load();
        let audio_engine = AudioEngine::new();
        let audio = Rc::new(RefCell::new(audio_engine));

        let hotkey_manager = HotkeyManager::new().unwrap_or_else(|e| {
            panic!("Менеджер горячих клавиш недоступен: {e}");
        });
        let hotkeys = Rc::new(RefCell::new(hotkey_manager));

        let devices = audio::AudioEngine::list_output_devices();

        // Загружаем сохранённые устройства из конфига
        let selected_main_device = if !config.output_devices.is_empty() {
            let name = &config.output_devices[0].name;
            match name {
                None => 0,
                Some(n) => devices.iter().position(|d| d.name == *n).unwrap_or(0),
            }
        } else {
            0
        };

        let selected_monitor_device = if config.output_devices.len() > 1 {
            let name = &config.output_devices[1].name;
            match name {
                None => 0,
                Some(n) => devices
                    .iter()
                    .position(|d| d.name == *n)
                    .map(|i| i + 1)
                    .unwrap_or(0),
            }
        } else {
            0
        };

        let mut app = Self {
            config,
            audio,
            hotkeys,
            devices,
            selected_main_device,
            selected_monitor_device,
            status: "Готово".to_string(),
            edit: None,
            capturing: None,
            dirty: false,
            last_device_refresh: Instant::now(),
            last_capture_time: Instant::now(),
            last_save: Instant::now(),
            theme_applied: false,
        };

        // Применяем настройки устройств при старте
        app.apply_devices_from_config();
        // Мастер-громкость задаётся движку один раз здесь.
        app.audio
            .borrow_mut()
            .set_master_volume(app.config.master_volume);
        app.audio
            .borrow_mut()
            .set_ptt_hotkey(app.config.ptt_hotkey.clone());
        app.audio
            .borrow_mut()
            .set_ptt_release_delay(app.config.ptt_release_delay_ms);
        app.register_all_hotkeys();
        app
    }

    fn apply_devices_from_config(&mut self) {
        self.audio.borrow_mut().remove_all_devices();

        let main_cfg = self.config.output_devices.first().cloned();
        let (main_name, main_volume) = match &main_cfg {
            Some(cfg) => (cfg.name.clone(), cfg.volume),
            None => (None, 1.0),
        };
        if let Err(e) =
            self.audio
                .borrow_mut()
                .add_device("main", main_name.as_deref(), main_volume, true)
        {
            self.status = format!("Ошибка осн. устройства: {e}");
        }

        let monitor_cfg = self.config.output_devices.get(1).cloned();
        if let Some(dev_cfg) = monitor_cfg {
            if dev_cfg.enabled {
                if let Err(e) = self.audio.borrow_mut().add_device(
                    "monitor",
                    dev_cfg.name.as_deref(),
                    dev_cfg.volume,
                    true,
                ) {
                    self.status = format!("Мониторинг: {e}");
                }
            }
        }
    }

    fn apply_main_device(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        if let Some(device) = self.devices.get(self.selected_main_device) {
            let name = if device.is_default {
                None
            } else {
                Some(device.name.as_str())
            };
            let volume = if !self.config.output_devices.is_empty() {
                self.config.output_devices[0].volume
            } else {
                1.0
            };
            if let Err(e) = self
                .audio
                .borrow_mut()
                .add_device("main", name, volume, true)
            {
                self.status = format!("Ошибка осн. устройства: {e}");
            }

            if !self.config.output_devices.is_empty() {
                self.config.output_devices[0].name = name.map(|s| s.to_string());
                self.config.output_devices[0].volume = volume;
                self.config.output_devices[0].enabled = true;
            } else {
                self.config
                    .output_devices
                    .push(crate::config::OutputDeviceConfig {
                        name: name.map(|s| s.to_string()),
                        volume,
                        enabled: true,
                    });
            }
            self.dirty = true;
        }
    }

    fn apply_monitoring_device(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        if self.selected_monitor_device == 0 {
            self.audio.borrow_mut().remove_device("monitor");
            if self.config.output_devices.len() > 1 {
                self.config.output_devices[1].enabled = false;
            }
        } else if let Some(dev) = self.devices.get(self.selected_monitor_device - 1) {
            let volume = if self.config.output_devices.len() > 1 {
                self.config.output_devices[1].volume
            } else {
                1.0
            };
            let name = if dev.is_default {
                None
            } else {
                Some(dev.name.as_str())
            };
            let saved_name = name.map(str::to_string);
            if let Err(e) = self
                .audio
                .borrow_mut()
                .add_device("monitor", name, volume, true)
            {
                self.status = format!("Мониторинг: {e}");
                self.selected_monitor_device = 0;
                if self.config.output_devices.len() > 1 {
                    self.config.output_devices[1].enabled = false;
                }
            } else {
                if self.config.output_devices.len() > 1 {
                    self.config.output_devices[1].name = saved_name;
                    self.config.output_devices[1].volume = volume;
                    self.config.output_devices[1].enabled = true;
                } else {
                    self.config
                        .output_devices
                        .push(crate::config::OutputDeviceConfig {
                            name: saved_name,
                            volume,
                            enabled: true,
                        });
                }
            }
        }
        self.dirty = true;
    }

    fn register_all_hotkeys(&mut self) {
        let mut hk = self.hotkeys.borrow_mut();
        hk.unregister_all();
        let entries: Vec<(u32, Option<HotkeyConfig>)> = self
            .config
            .sounds
            .iter()
            .map(|s| (s.id, s.hotkey.clone()))
            .collect();
        for (id, hotkey) in &entries {
            if let Some(h) = hotkey {
                if let Err(e) = hk.register(HotkeyAction::Play(*id), h) {
                    log::warn!("Не удалось зарегистрировать клавишу для записи {id}: {e}");
                }
            }
        }
        if let Some(h) = &self.config.stop_all_hotkey {
            if let Err(e) = hk.register(HotkeyAction::StopAll, h) {
                log::warn!("Не удалось зарегистрировать клавишу Стоп: {e}");
            }
        }
    }

    fn play_entry(&mut self, index: usize) {
        let entry = match self.config.sounds.get(index) {
            Some(e) => e.clone(),
            None => return,
        };
        let mut audio = self.audio.borrow_mut();
        match audio.play(
            entry.id,
            &entry.path,
            &entry.name,
            entry.volume,
            self.config.stop_on_replay,
        ) {
            Ok(true) => self.status = format!("▶ {}", entry.name),
            Ok(false) => self.status = format!("Уже играет: {}", entry.name),
            Err(e) => self.status = format!("Ошибка: {e}"),
        }
    }

    fn stop_entry(&mut self, index: usize) {
        if let Some(entry) = self.config.sounds.get(index) {
            self.audio.borrow_mut().stop(entry.id);
            self.status = format!("■ {}", entry.name);
        }
    }

    fn start_capture(&mut self, target: CaptureTarget) {
        self.capturing = Some(target);
        self.status = "Нажмите любую клавишу (Esc — отмена, Backspace — убрать)".into();
    }

    /// Сохраняет конфиг, если он изменился. Запись — не чаще раза в 1.5 с.
    fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        if self.last_save.elapsed() < std::time::Duration::from_millis(1500) {
            return;
        }
        self.last_save = Instant::now();
        match self.config.save() {
            Ok(()) => self.dirty = false,
            Err(e) => self.status = format!("Не удалось сохранить: {e}"),
        }
    }

    /// Раз в 5 секунд обновляет список устройств.
    fn refresh_devices_if_needed(&mut self) {
        if self.last_device_refresh.elapsed().as_secs() < 5 {
            return;
        }
        self.last_device_refresh = Instant::now();

        let new_devices = AudioEngine::list_output_devices();
        let changed = new_devices.len() != self.devices.len()
            || new_devices
                .iter()
                .zip(self.devices.iter())
                .any(|(a, b)| a.name != b.name);
        if !changed {
            return;
        }
        log::info!("Список устройств изменился, обновляем.");

        self.devices = new_devices;

        self.selected_main_device = if !self.config.output_devices.is_empty() {
            let name = &self.config.output_devices[0].name;
            match name {
                None => 0,
                Some(n) => self.devices.iter().position(|d| &d.name == n).unwrap_or(0),
            }
        } else {
            0
        };

        self.selected_monitor_device = if self.config.output_devices.len() > 1 {
            let name = &self.config.output_devices[1].name;
            match name {
                None => 0,
                Some(n) => self
                    .devices
                    .iter()
                    .position(|d| &d.name == n)
                    .map(|i| i + 1)
                    .unwrap_or(0),
            }
        } else {
            0
        };
        self.status = "Список устройств обновлён".into();
    }
}

impl eframe::App for ZvukopadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Применяем тему «спокойный тёмный Material 3» один раз за сессию.
        if !self.theme_applied {
            theme::apply_visuals(ctx, true);
            self.theme_applied = true;
        }

        // 0. Периодически обновляем список устройств (каждые 5 секунд).
        self.refresh_devices_if_needed();

        // 1. Опрашиваем глобальные горячие клавиши.
        let triggered = self.hotkeys.borrow().poll();
        for action in triggered {
            match action {
                HotkeyAction::Play(entry_id) => {
                    if let Some(idx) = self.config.sounds.iter().position(|s| s.id == entry_id) {
                        self.play_entry(idx);
                    }
                }
                HotkeyAction::StopAll => {
                    self.audio.borrow_mut().stop_all();
                    self.status = "■ Все остановлено (гл. клавиша)".into();
                }
                HotkeyAction::Ptt => {}
            }
        }

        // 2. Обработка захвата клавиши.
        self.handle_capture(ctx);

        // 3. Чистка завершившихся звуков + проверка PTT.
        self.audio.borrow_mut().cleanup_finished();

        // Пока играет звук (или ждём отпускания PTT) обновляемся 20 раз в
        // секунду, в покое — 4 раза: интерфейс не жжёт CPU вхолостую.
        let busy = self.audio.borrow().needs_periodic_wakeup() || self.capturing.is_some();
        ctx.request_repaint_after(std::time::Duration::from_millis(if busy {
            50
        } else {
            250
        }));

        // 4. Самая нижняя панель — ссылка на донат (слева).
        egui::TopBottomPanel::bottom("donate_bar").show(ctx, |ui| {
            ui.add_space(PAD_SCREEN);
            ui.horizontal(|ui| {
                ui.add_space(PAD_SCREEN);
                ui.hyperlink_to(
                    egui::RichText::new("❤ Поддержать автора (DonationAlerts)")
                        .color(dark::accent()),
                    "https://www.donationalerts.com/r/didimoon",
                );
            });
            ui.add_space(PAD_SCREEN);
        });

        // 5. Статус-бар.
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::NONE
                    .fill(dark::bg())
                    .inner_margin(egui::Margin::symmetric(16, 0)),
            )
            .show(ctx, |ui| {
                card_frame().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&self.status)
                                .size(FONT_BODY)
                                .color(dark::secondary()),
                        )
                        .wrap(),
                    );

                    let playing = self.audio.borrow().playing_names();
                    if !playing.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("Играет:")
                                    .size(FONT_BODY)
                                    .color(dark::muted()),
                            );
                            ui.label(
                                egui::RichText::new(
                                    playing
                                        .iter()
                                        .map(|(_, n)| n.clone())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                )
                                .size(FONT_BODY)
                                .color(dark::text()),
                            );
                        });
                    }
                });
            });

        // 6. Настройки (правая панель).
        egui::SidePanel::right("settings")
            .resizable(false)
            .exact_width(380.0)
            .frame(
                egui::Frame::NONE
                    .fill(dark::bg())
                    .inner_margin(egui::Margin::symmetric(16, 16)),
            )
            .show(ctx, |ui| {
                self.draw_settings(ui);
            });

        // 7. Список звуков (центр).
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(dark::bg())
                    .inner_margin(egui::Margin::symmetric(16, 16)),
            )
            .show(ctx, |ui| {
                card_frame().show(ui, |ui| {
                    self.draw_sounds_list(ui);
                });
            });

        // 8. Модальное окно редактирования.
        self.draw_edit_dialog(ctx);

        // 9. Сохраняем изменения.
        self.save_if_dirty();
    }

    /// Финальный сброс изменений на диск при выходе.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.dirty {
            if let Err(e) = self.config.save() {
                log::error!("Не удалось сохранить конфигурацию при выходе: {e}");
            }
            self.dirty = false;
        }
    }
}

impl ZvukopadApp {
    fn handle_capture(&mut self, _ctx: &egui::Context) {
        if self.last_capture_time.elapsed() < std::time::Duration::from_millis(150) {
            return;
        }

        let Some(capture) = self.capturing else {
            return;
        };

        if crate::kb_capture::is_escape_pressed() {
            self.capturing = None;
            self.status = "Отменено".into();
            return;
        }
        if crate::kb_capture::is_backspace_pressed() {
            match capture {
                CaptureTarget::Sound(idx) => {
                    if let Some(e) = self.config.sounds.get_mut(idx) {
                        self.hotkeys
                            .borrow_mut()
                            .unregister(HotkeyAction::Play(e.id));
                        e.hotkey = None;
                    }
                    self.dirty = true;
                }
                CaptureTarget::StopAll => {
                    if self.config.stop_all_hotkey.take().is_some() {
                        self.hotkeys.borrow_mut().unregister(HotkeyAction::StopAll);
                    }
                    self.dirty = true;
                }
                CaptureTarget::Ptt => {
                    self.config.ptt_hotkey = None;
                    self.audio.borrow_mut().set_ptt_hotkey(None);
                    self.dirty = true;
                }
            }
            self.capturing = None;
            self.status = "Клавиша убрана".into();
            return;
        }

        let Some(captured) = crate::kb_capture::poll_pressed() else {
            return;
        };

        let hk = HotkeyConfig::new(captured.modifiers, captured.code);
        let apply_result = match capture {
            CaptureTarget::Sound(idx) => {
                if let Some(e) = self.config.sounds.get_mut(idx) {
                    match self
                        .hotkeys
                        .borrow_mut()
                        .register(HotkeyAction::Play(e.id), &hk)
                    {
                        Ok(()) => {
                            e.hotkey = Some(hk);
                            self.dirty = true;
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Err("Запись не найдена".into())
                }
            }
            CaptureTarget::StopAll => match self
                .hotkeys
                .borrow_mut()
                .register(HotkeyAction::StopAll, &hk)
            {
                Ok(()) => {
                    self.config.stop_all_hotkey = Some(hk);
                    self.dirty = true;
                    Ok(())
                }
                Err(e) => Err(e),
            },
            CaptureTarget::Ptt => {
                self.audio.borrow_mut().set_ptt_hotkey(Some(hk.clone()));
                self.config.ptt_hotkey = Some(hk);
                self.dirty = true;
                Ok(())
            }
        };
        match apply_result {
            Ok(()) => self.status = "Клавиша назначена".into(),
            Err(e) => self.status = e,
        }
        self.capturing = None;
        self.last_capture_time = Instant::now();
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.draw_settings_inner(ui);
            });
    }

    fn draw_settings_inner(&mut self, ui: &mut egui::Ui) {
        card_title(ui, "⚙ Настройки");

        // --- Устройства вывода, громкость и поведение ---
        settings_card().show(ui, |ui| {
            caption(ui, "Основное — сюда идёт звук (обычно CABLE Input)");
            ui.add_space(4.0);

            let main_display = self
                .devices
                .get(self.selected_main_device)
                .map(|d| d.name.as_str())
                .unwrap_or(DEFAULT_DEVICE_NAME);

            egui::ComboBox::from_id_salt("main_device")
                .width(ui.available_width())
                .selected_text(main_display)
                .show_ui(ui, |ui| {
                    let mut new_main_device = self.selected_main_device;
                    for (i, device) in self.devices.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_main_device == i, &device.name)
                            .clicked()
                        {
                            new_main_device = i;
                        }
                    }
                    if new_main_device != self.selected_main_device {
                        self.selected_main_device = new_main_device;
                        self.dirty = true;
                        self.apply_main_device();
                    }
                });

            let mut mv = if !self.config.output_devices.is_empty() {
                self.config.output_devices[0].volume
            } else {
                1.0
            };
            if ui
                .add(egui::Slider::new(&mut mv, 0.0..=1.0).text("Громкость"))
                .changed()
            {
                if !self.config.output_devices.is_empty() {
                    self.config.output_devices[0].volume = mv;
                } else {
                    self.config
                        .output_devices
                        .push(crate::config::OutputDeviceConfig {
                            name: None,
                            volume: mv,
                            enabled: true,
                        });
                }
                self.audio.borrow_mut().set_device_volume("main", mv).ok();
                self.dirty = true;
            }

            ui.add_space(8.0);
            caption(ui, "Мониторинг — дублировать звук, например в наушники");
            ui.add_space(4.0);

            let mut mon_names = vec![NO_DEVICE_NAME.to_string()];
            mon_names.extend(self.devices.iter().map(|d| d.name.clone()));
            let mon_display = if self.selected_monitor_device == 0 {
                NO_DEVICE_NAME
            } else {
                mon_names
                    .get(self.selected_monitor_device)
                    .map(|s| s.as_str())
                    .unwrap_or(NO_DEVICE_NAME)
            };

            egui::ComboBox::from_id_salt("monitor_device")
                .width(ui.available_width())
                .selected_text(mon_display)
                .show_ui(ui, |ui| {
                    let mut new_monitor_device = self.selected_monitor_device;
                    for (i, name) in mon_names.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_monitor_device == i, name)
                            .clicked()
                        {
                            new_monitor_device = i;
                        }
                    }
                    if new_monitor_device != self.selected_monitor_device {
                        self.selected_monitor_device = new_monitor_device;
                        self.dirty = true;
                        self.apply_monitoring_device();
                    }
                });

            if self.selected_monitor_device > 0 {
                let mut mv = if self.config.output_devices.len() > 1 {
                    self.config.output_devices[1].volume
                } else {
                    1.0
                };
                if ui
                    .add(egui::Slider::new(&mut mv, 0.0..=1.0).text("Громкость мониторинга"))
                    .changed()
                {
                    if self.config.output_devices.len() > 1 {
                        self.config.output_devices[1].volume = mv;
                    } else {
                        self.config
                            .output_devices
                            .push(crate::config::OutputDeviceConfig {
                                name: None,
                                volume: mv,
                                enabled: true,
                            });
                    }
                    self.audio
                        .borrow_mut()
                        .set_device_volume("monitor", mv)
                        .ok();
                    self.dirty = true;
                }
            }

            ui.add_space(8.0);
            let mut vol = self.config.master_volume;
            if ui
                .add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Мастер"))
                .changed()
            {
                self.config.master_volume = vol;
                self.dirty = true;
                self.audio.borrow_mut().set_master_volume(vol);
            }

            ui.add_space(4.0);
            let mut stop_on_replay = self.config.stop_on_replay;
            ui.checkbox(&mut stop_on_replay, "Перезапуск при повторном нажатии");
            if stop_on_replay != self.config.stop_on_replay {
                self.config.stop_on_replay = stop_on_replay;
                self.dirty = true;
            }
        });

        // --- Горячие клавиши ---
        settings_card().show(ui, |ui| {
            card_title(ui, "⌨ Горячие клавиши");
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                caption(ui, "Остановить всё");
                let stop_text = self
                    .config
                    .stop_all_hotkey
                    .as_ref()
                    .map(hotkeys::hotkey_display)
                    .unwrap_or_else(|| "— не назначена —".to_string());
                let capturing_stop = self
                    .capturing
                    .map(|c| c == CaptureTarget::StopAll)
                    .unwrap_or(false);
                let btn_text = if capturing_stop {
                    "Нажмите... (Esc)".to_string()
                } else {
                    stop_text
                };
                if ui.button(btn_text).clicked() && !capturing_stop {
                    self.start_capture(CaptureTarget::StopAll);
                }
                if self.config.stop_all_hotkey.is_some() && small_text_button(ui, "×").clicked() {
                    self.config.stop_all_hotkey = None;
                    self.hotkeys.borrow_mut().unregister(HotkeyAction::StopAll);
                    self.dirty = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                caption(ui, "Push-To-Talk");
                let ptt_text = self
                    .config
                    .ptt_hotkey
                    .as_ref()
                    .map(hotkeys::hotkey_display)
                    .unwrap_or_else(|| "— не назначена —".to_string());
                let capturing_ptt = self
                    .capturing
                    .map(|c| c == CaptureTarget::Ptt)
                    .unwrap_or(false);
                let btn_text = if capturing_ptt {
                    "Нажмите... (Esc)".to_string()
                } else {
                    ptt_text
                };
                if ui.button(btn_text).clicked() && !capturing_ptt {
                    self.start_capture(CaptureTarget::Ptt);
                }
                if self.config.ptt_hotkey.is_some() && small_text_button(ui, "×").clicked() {
                    self.config.ptt_hotkey = None;
                    self.audio.borrow_mut().set_ptt_hotkey(None);
                    self.dirty = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                caption(ui, "Отпускание, мс:");
                let resp = ui.add(
                    egui::DragValue::new(&mut self.config.ptt_release_delay_ms).range(0..=5000),
                );
                if resp.changed() {
                    self.dirty = true;
                    self.audio
                        .borrow_mut()
                        .set_ptt_release_delay(self.config.ptt_release_delay_ms);
                }
            });

            ui.add_space(4.0);
            caption(ui, "PTT нажимает клавишу микрофона игры, пока звучит звук.");
        });

        // --- Подсказка ---
        self.draw_help(ui);
    }

    /// Центральная панель: список звуков.
    fn draw_sounds_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            app_title(ui, "Звукопад");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, "+ Добавить звук").clicked() {
                    self.edit = Some(PendingEdit {
                        index: self.config.sounds.len(),
                        name: String::new(),
                        path: String::new(),
                        volume: 1.0,
                    });
                }
                if text_button(ui, "■ Остановить всё").clicked() {
                    self.audio.borrow_mut().stop_all();
                    self.status = "Всё остановлено".into();
                }
            });
        });
        ui.add_space(4.0);
        secondary_text(
            ui,
            "Звуковая панель — нажмите ▶ или используйте горячие клавиши",
        );
        ui.add_space(PAD_BETWEEN);

        if self.config.sounds.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                caption(ui, "Список пуст.");
                ui.add_space(8.0);
                caption(ui, "Нажмите «+ Добавить звук» вверху справа.");
            });
            return;
        }

        let available_width = ui.available_width();
        // Карточки не должны быть уже ~300 px: два столбца при окне 1200 px.
        let min_card_w = 300.0;
        let cols = (((available_width + PAD_BETWEEN) / (min_card_w + PAD_BETWEEN)).floor()
            as usize)
            .max(1);
        let cell_width = ((available_width - PAD_BETWEEN * (cols.saturating_sub(1) as f32))
            / cols as f32)
            .max(200.0);

        // Список звуков прокручивается: иначе при большом количестве записей
        // нижние карточки просто обрезались бы краем панели.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(available_width);
                egui::Grid::new("sound_grid")
                    .num_columns(cols)
                    .spacing([PAD_BETWEEN, PAD_BETWEEN])
                    .show(ui, |ui| {
                        let mut to_play: Option<usize> = None;
                        let mut to_stop: Option<usize> = None;
                        let mut to_capture: Option<usize> = None;
                        let mut to_remove: Option<usize> = None;
                        let mut to_edit: Option<usize> = None;
                        let mut to_clear_hotkey: Option<usize> = None;
                        let mut volume_changes: Vec<(usize, f32)> = Vec::new();

                        let playing_ids: Vec<u32> = self
                            .audio
                            .borrow()
                            .playing_names()
                            .into_iter()
                            .map(|(id, _)| id)
                            .collect();

                        for (i, entry) in self.config.sounds.iter().enumerate() {
                            let playing = playing_ids.contains(&entry.id);
                            let hotkey_text = entry
                                .hotkey
                                .as_ref()
                                .map(hotkeys::hotkey_display)
                                .unwrap_or_else(|| "—".to_string());

                            // Карточка записи без внешнего отступа — расстояние между
                            // карточками задаёт Grid spacing.
                            egui::Frame::NONE
                                .fill(dark::block())
                                .corner_radius(CornerRadius::same(theme::CORNER_FIELD))
                                .stroke(egui::Stroke::new(1.0_f32, dark::border()))
                                .inner_margin(egui::vec2(PAD_CARD, PAD_CARD))
                                .show(ui, |ui| {
                                    ui.set_width(cell_width - PAD_CARD * 2.0);
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            let label = if playing {
                                                format!("■ {}", entry.name)
                                            } else {
                                                format!("▶ {}", entry.name)
                                            };
                                            let btn = egui::Button::new(
                                                egui::RichText::new(label)
                                                    .size(FONT_BODY)
                                                    .color(dark::accent_text()),
                                            )
                                            .fill(if playing {
                                                dark::success()
                                            } else {
                                                dark::accent()
                                            })
                                            .stroke(egui::Stroke::new(
                                                1.0_f32,
                                                if playing {
                                                    dark::success()
                                                } else {
                                                    dark::accent_border()
                                                },
                                            ))
                                            .corner_radius(CornerRadius::same(theme::CORNER_PILL));
                                            if ui.add(btn).clicked() {
                                                if playing {
                                                    to_stop = Some(i);
                                                } else {
                                                    to_play = Some(i);
                                                }
                                            }
                                            if small_text_button(ui, "Изменить").clicked() {
                                                to_edit = Some(i);
                                            }
                                            if small_text_button(ui, "Удалить").clicked() {
                                                to_remove = Some(i);
                                            }
                                        });
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            caption(ui, "Клавиша:");
                                            let capturing_now = self
                                                .capturing
                                                .map(|c| c == CaptureTarget::Sound(i))
                                                .unwrap_or(false);
                                            let btn_text = if capturing_now {
                                                "Нажмите… (Esc)".to_string()
                                            } else {
                                                hotkey_text
                                            };
                                            if ui.button(btn_text).clicked() && !capturing_now {
                                                to_capture = Some(i);
                                            }
                                            if entry.hotkey.is_some()
                                                && small_text_button(ui, "×").clicked()
                                            {
                                                to_clear_hotkey = Some(i);
                                            }
                                        });
                                        ui.add_space(8.0);
                                        let mut v = entry.volume;
                                        let resp = ui.add(
                                            egui::Slider::new(&mut v, 0.0..=1.0)
                                                .show_value(false)
                                                .text("громкость"),
                                        );
                                        if resp.changed() {
                                            volume_changes.push((i, v));
                                        }
                                        ui.add_space(4.0);
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&entry.path)
                                                    .small()
                                                    .color(dark::muted()),
                                            )
                                            .truncate(),
                                        );

                                        // Точка статуса: играет или остановлен.
                                        ui.add_space(8.0);
                                        if playing {
                                            status_row(ui, dark::success(), "Играет");
                                        } else {
                                            status_row(ui, dark::muted(), "Готов");
                                        }
                                    });
                                });
                            // Grid сам не переносит строки: закрываем строку только
                            // на последней карточке ряда, иначе все звуки встают в
                            // один столбец.
                            if (i + 1) % cols == 0 || i + 1 == self.config.sounds.len() {
                                ui.end_row();
                            }
                        }

                        // Применяем отложенные действия.
                        if let Some(i) = to_play {
                            self.play_entry(i);
                        }
                        if let Some(i) = to_stop {
                            self.stop_entry(i);
                        }
                        if let Some(i) = to_capture {
                            self.start_capture(CaptureTarget::Sound(i));
                        }
                        if let Some(i) = to_remove {
                            self.remove_entry(i);
                        }
                        if let Some(i) = to_edit {
                            if let Some(e) = self.config.sounds.get(i).cloned() {
                                self.edit = Some(PendingEdit {
                                    index: i,
                                    name: e.name,
                                    path: e.path,
                                    volume: e.volume,
                                });
                            }
                        }
                        if let Some(i) = to_clear_hotkey {
                            if let Some(e) = self.config.sounds.get_mut(i) {
                                self.hotkeys
                                    .borrow_mut()
                                    .unregister(HotkeyAction::Play(e.id));
                                e.hotkey = None;
                                self.dirty = true;
                            }
                        }
                        for (i, v) in volume_changes {
                            if let Some(e) = self.config.sounds.get_mut(i) {
                                e.volume = v;
                                self.audio.borrow_mut().set_entry_volume(e.id, v);
                                self.dirty = true;
                            }
                        }
                    });
            });
    }

    fn remove_entry(&mut self, index: usize) {
        if let Some(entry) = self.config.sounds.get(index) {
            self.hotkeys
                .borrow_mut()
                .unregister(HotkeyAction::Play(entry.id));
            // Удалённая запись не должна продолжать играть в фоне.
            self.audio.borrow_mut().stop(entry.id);
        }
        self.config.sounds.remove(index);
        self.dirty = true;
        self.status = "Запись удалена".into();
    }

    fn draw_edit_dialog(&mut self, ctx: &egui::Context) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;

        egui::Window::new("Звуковая запись")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                card_frame().show(ui, |ui| {
                    caption(ui, "Название:");
                    ui.text_edit_singleline(&mut edit.name);

                    ui.add_space(PAD_BETWEEN);
                    caption(ui, "Путь к звуковому файлу (mp3, wav, ogg, flac):");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut edit.path);
                        if ui.button("Обзор…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Звук", &["mp3", "wav", "ogg", "flac", "m4a", "aac"])
                                .pick_file()
                            {
                                edit.path = path.display().to_string();
                                if edit.name.trim().is_empty() {
                                    if let Some(stem) = path.file_stem() {
                                        edit.name = stem.to_string_lossy().to_string();
                                    }
                                }
                            }
                        }
                    });

                    ui.add_space(PAD_BETWEEN);
                    caption(ui, "Громкость:");
                    ui.add(egui::Slider::new(&mut edit.volume, 0.0..=1.0));

                    ui.add_space(PAD_BETWEEN);
                    ui.horizontal(|ui| {
                        if pill_button(ui, "Сохранить").clicked() {
                            apply = true;
                        }
                        if text_button(ui, "Отмена").clicked() {
                            cancel = true;
                        }
                    });
                });
            });

        if !open || cancel {
            self.edit = None;
        }
        if apply {
            let edit = self.edit.take().expect("edit должен быть Some при apply");
            let new_entry = SoundEntry {
                id: self
                    .config
                    .sounds
                    .get(edit.index)
                    .map(|e| e.id)
                    .unwrap_or_else(|| self.config.next_id()),
                name: edit.name.trim().to_string(),
                path: edit.path.trim().to_string(),
                volume: edit.volume,
                hotkey: None,
            };
            if edit.index >= self.config.sounds.len() {
                self.config.sounds.push(new_entry);
                self.status = "Звук добавлен".into();
            } else {
                let prev_hk = self.config.sounds[edit.index].hotkey.clone();
                let mut updated = new_entry;
                updated.hotkey = prev_hk;
                self.config.sounds[edit.index] = updated;
                self.status = "Звук обновлён".into();
            }
            self.dirty = true;
        }
    }

    /// Подробная подсказка с инструкцией и ссылкой на VB-Cable.
    fn draw_help(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Как пользоваться Звукопадом")
            .show(ui, |ui| {
                card_frame().show(ui, |ui| {
                    body(ui, "1. Добавьте звуки через «+ Добавить звук».");
                    ui.add_space(4.0);
                    body(ui, "2. Назначьте горячие клавиши — любая комбинация или просто одна клавиша (например Numpad1).");
                    ui.add_space(4.0);
                    body(ui, "3. Чтобы звук шёл в микрофон (Discord, OBS, игры):");
                    ui.add_space(2.0);
                    body(ui, "   • Скачайте и установите VB-Audio Virtual Cable:");
                    ui.hyperlink_to(
                        "   https://vb-audio.com/Cable/",
                        "https://vb-audio.com/Cable/",
                    );
                    ui.add_space(2.0);
                    body(ui, "   • В настройках Звукопада:");
                    body(ui, "      — Основное устройство: выберите «CABLE Input (VB-Audio…)»");
                    body(ui, "      — Устройство мониторинга: выберите ваши наушники (необязательно!)");
                    body(ui, "   • В Discord / OBS / игре установите микрофон: «CABLE Output»");
                    ui.add_space(4.0);
                    body(ui, "4. Для автоматического включения микрофона:");
                    body(ui, "   • В разделе «Имитация микрофона» укажите клавишу,");
                    body(ui, "      которая открывает голосовой чат в вашей игре.");
                    body(ui, "   • Во время воспроизведения Звукопад автоматически");
                    body(ui, "      нажмёт эту клавишу — игра подумает, что вы говорите.");
                    ui.add_space(4.0);
                    body(ui, "5. Глобальная клавиша «Остановить всё» позволяет мгновенно");
                    body(ui, "   остановить все играющие звуки одной кнопкой.");
                });
            });
    }
}

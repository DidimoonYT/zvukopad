//! Графический интерфейс приложения на egui.
//!
//! Главное окно: список звуковых записей, выбор устройств вывода,
//! мониторинг, мастер-громкость, глобальные горячие клавиши (стоп/PTT)
//! и подробная подсказка по использованию.

use crate::audio::{self, AudioEngine, DEFAULT_DEVICE_NAME, NO_DEVICE_NAME};
use crate::config::{Config, HotkeyConfig, SoundEntry};
use crate::hotkeys::{self, HotkeyAction, HotkeyManager};
use eframe::egui;
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
                Some(n) => devices.iter().position(|d| d.name == *n).map(|i| i + 1).unwrap_or(0),
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
        };

        // Применяем настройки устройств при старте
        app.apply_devices_from_config();
        app.audio.borrow_mut().set_ptt_hotkey(app.config.ptt_hotkey.clone());
        app.audio.borrow_mut().set_ptt_release_delay(app.config.ptt_release_delay_ms);
        app.register_all_hotkeys();
        app
    }

    fn apply_devices_from_config(&mut self) {
        // Очищаем текущие устройства
        self.audio.borrow_mut().stop_all();
        
        // Основное устройство (индекс 0 в конфиге)
        if !self.config.output_devices.is_empty() {
            let dev_cfg = &self.config.output_devices[0];
            if dev_cfg.enabled {
                let name = dev_cfg.name.as_deref();
                let volume = dev_cfg.volume;
                if let Err(e) = self.audio.borrow_mut().add_device("main", name, volume, true) {
                    self.status = format!("Ошибка осн. устройства: {e}");
                }
            }
        }
        
        // Мониторинг (индекс 1 в конфиге)
        if self.config.output_devices.len() > 1 {
            let dev_cfg = &self.config.output_devices[1];
            if dev_cfg.enabled {
                let name = dev_cfg.name.as_deref();
                let volume = dev_cfg.volume;
                if let Err(e) = self.audio.borrow_mut().add_device("monitor", name, volume, true) {
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
            let name = if device.is_default { None } else { Some(device.name.as_str()) };
            let volume = if !self.config.output_devices.is_empty() {
                self.config.output_devices[0].volume
            } else {
                1.0
            };
            if let Err(e) = self.audio.borrow_mut().add_device("main", name, volume, true) {
                self.status = format!("Ошибка осн. устройства: {e}");
            }
            
            // Сохраняем в конфиг (output_devices[0])
            if !self.config.output_devices.is_empty() {
                self.config.output_devices[0].name = name.map(|s| s.to_string());
                self.config.output_devices[0].volume = volume;
                self.config.output_devices[0].enabled = true;
            } else {
                self.config.output_devices.push(crate::config::OutputDeviceConfig {
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
            // "нет"
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
            if let Err(e) = self
                .audio
                .borrow_mut()
                .add_device("monitor", Some(&dev.name), volume, true)
            {
                self.status = format!("Мониторинг: {e}");
                // Сбрасываем выбор мониторинга в конфиге, т.к. устройство не открылось.
                self.selected_monitor_device = 0;
                if self.config.output_devices.len() > 1 {
                    self.config.output_devices[1].enabled = false;
                }
            } else {
                // Сохраняем в конфиг (output_devices[1])
                if self.config.output_devices.len() > 1 {
                    self.config.output_devices[1].name = Some(dev.name.clone());
                    self.config.output_devices[1].volume = volume;
                    self.config.output_devices[1].enabled = true;
                } else {
                    self.config.output_devices.push(crate::config::OutputDeviceConfig {
                        name: Some(dev.name.clone()),
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
        // Записи звуков.
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
        // Глобальная клавиша «Стоп».
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
            self.config.master_volume,
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
            self.status = format!("⏹ {}", entry.name);
        }
    }

    fn start_capture(&mut self, target: CaptureTarget) {
        self.capturing = Some(target);
        self.status = "Нажмите любую клавишу (Esc — отмена, Backspace — убрать)".into();
    }

    fn save_if_dirty(&mut self) {
        if self.dirty {
            if let Err(e) = self.config.save() {
                self.status = format!("Не удалось сохранить: {e}");
            }
            self.dirty = false;
        }
    }

    /// Раз в 5 секунд обновляет список устройств. Если набор устройств изменился,
    /// пересинхронизирует выбранные индексы с конфигом.
    fn refresh_devices_if_needed(&mut self) {
        if self.last_device_refresh.elapsed().as_secs() < 5 {
            return;
        }
        self.last_device_refresh = Instant::now();

        let new_devices = AudioEngine::list_output_devices();
        // Быстрое сравнение по набору имён.
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
        
        // Пересинхронизируем индексы по именам из конфига (output_devices).
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
                Some(n) => self.devices.iter().position(|d| &d.name == n).map(|i| i + 1).unwrap_or(0),
            }
        } else {
            0
        };
        self.status = "Список устройств обновлён".into();
    }
}

impl eframe::App for ZvukopadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    self.status = "⏹ Все остановлено (гл. клавиша)".into();
                }
                HotkeyAction::Ptt => {
                    // PTT-клавиша обрабатывается отдельно — не через глобальный хоткей,
                    // а через прямую имитацию в audio.rs. Но если зарегистрирована,
                    // игнорируем здесь (см. настройки).
                }
            }
        }

        // 2. Обработка захвата клавиши.
        self.handle_capture(ctx);

        // 3. Чистка завершившихся звуков + проверка PTT.
        self.audio.borrow_mut().cleanup_finished();

        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        // 4. Статус-бар.
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                let playing = self.audio.borrow().playing_names();
                if !playing.is_empty() {
                    ui.separator();
                    ui.label("Играет:");
                    ui.label(playing.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(", "));
                }
            });
        });

        // 5. Настройки (правая панель).
        egui::SidePanel::right("settings").show(ctx, |ui| {
            self.draw_settings(ui);
        });

        // 6. Список звуков (центр).
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_sounds_list(ui);
        });

        // 7. Модальное окно редактирования.
        self.draw_edit_dialog(ctx);

        // 8. Сохраняем изменения.
        self.save_if_dirty();
    }
}

impl ZvukopadApp {
    fn handle_capture(&mut self, _ctx: &egui::Context) {
        // Debounce: не обрабатываем захват чаще чем раз в 150 мс,
        // чтобы не поймать отпускание предыдущей клавиши.
        if self.last_capture_time.elapsed() < std::time::Duration::from_millis(150) {
            return;
        }

        let Some(capture) = self.capturing else {
            return;
        };

        // Отмена / очистка.
        if crate::kb_capture::is_escape_pressed() {
            self.capturing = None;
            self.status = "Отменено".into();
            return;
        }
        if crate::kb_capture::is_backspace_pressed() {
            match capture {
                CaptureTarget::Sound(idx) => {
                    if let Some(e) = self.config.sounds.get_mut(idx) {
                        self.hotkeys.borrow_mut().unregister(HotkeyAction::Play(e.id));
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

        // Захват основной клавиши + модификаторов напрямую через WinAPI.
        // Это позволяет корректно различать Numpad и верхний ряд цифр.
        let Some(captured) = crate::kb_capture::poll_pressed() else {
            return; // ни одна основная клавиша не нажата
        };

        let hk = HotkeyConfig::new(captured.modifiers, captured.code);
        let apply_result = match capture {
            CaptureTarget::Sound(idx) => {
                if let Some(e) = self.config.sounds.get_mut(idx) {
                    match self.hotkeys.borrow_mut().register(HotkeyAction::Play(e.id), &hk) {
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
            CaptureTarget::StopAll => match self.hotkeys.borrow_mut().register(HotkeyAction::StopAll, &hk) {
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
        // Прокручиваемая область, чтобы все настройки помещались в окно.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.draw_settings_inner(ui);
            });
    }

    fn draw_settings_inner(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙ Настройки");
        ui.add_space(4.0);

        // --- Основное устройство вывода ---
                ui.collapsing("🔊 Устройства вывода", |ui| {
                    ui.label(
                        "Основное устройство: звук пойдёт на это устройство (обычно VB-Cable для передачи в Discord/игры).",
                    );

                    // Список всех устройств
                    let display = self
                        .devices
                        .get(self.selected_main_device)
                        .map(|d| d.name.as_str())
                        .unwrap_or(DEFAULT_DEVICE_NAME);

                    egui::ComboBox::from_label("")
                        .selected_text(display)
                        .show_ui(ui, |ui| {
                            let mut new_main_device = self.selected_main_device;
                            for (i, device) in self.devices.iter().enumerate() {
                                if ui.selectable_label(self.selected_main_device == i, &device.name).clicked() {
                                    new_main_device = i;
                                }
                            }
                            if new_main_device != self.selected_main_device {
                                self.selected_main_device = new_main_device;
                                self.dirty = true;
                                self.apply_main_device();
                            }
                        });

                    // Громкость основного устройства
                    let mut mv = if !self.config.output_devices.is_empty() {
                        self.config.output_devices[0].volume
                    } else {
                        1.0
                    };
                    if ui
                        .add(egui::Slider::new(&mut mv, 0.0..=1.0).text("Громкость устройства 🔊"))
                        .changed()
                    {
                        if !self.config.output_devices.is_empty() {
                            self.config.output_devices[0].volume = mv;
                        } else {
                            self.config.output_devices.push(crate::config::OutputDeviceConfig {
                                name: None,
                                volume: mv,
                                enabled: true,
                            });
                        }
                        self.audio.borrow_mut().set_device_volume("main", mv).ok();
                        self.dirty = true;
                    }
                });

        ui.add_space(4.0);

        // --- Устройство мониторинга ---
                ui.collapsing("🎧 Устройство мониторинга (доп. вывод)", |ui| {
                    ui.label(
                        "Дополнительное устройство для вывода звука. Можно выбрать
                         ЛЮБОЕ устройство (наушники, колонки, второй кабель и т.д.),
                         и звук будет идти на него одновременно с основным.",
                    );

                    // Список: первый элемент — "нет"
                    let mut mon_names = vec![NO_DEVICE_NAME.to_string()];
                    mon_names.extend(self.devices.iter().map(|d| d.name.clone()));

                    let display = if self.selected_monitor_device == 0 {
                        NO_DEVICE_NAME
                    } else {
                        mon_names
                            .get(self.selected_monitor_device)
                            .map(|s| s.as_str())
                            .unwrap_or(NO_DEVICE_NAME)
                    };

                    egui::ComboBox::from_label("")
                        .selected_text(display)
                        .show_ui(ui, |ui| {
                            let mut new_monitor_device = self.selected_monitor_device;
                            for (i, name) in mon_names.iter().enumerate() {
                                if ui.selectable_label(self.selected_monitor_device == i, name).clicked() {
                                    new_monitor_device = i;
                                }
                            }
                            if new_monitor_device != self.selected_monitor_device {
                                self.selected_monitor_device = new_monitor_device;
                                self.dirty = true;
                                self.apply_monitoring_device();
                            }
                        });

                    // Громкость мониторинга - показываем только если мониторинг включён
                    if self.selected_monitor_device > 0 {
                        let mut mv = if self.config.output_devices.len() > 1 {
                            self.config.output_devices[1].volume
                        } else {
                            1.0
                        };
                        if ui
                            .add(egui::Slider::new(&mut mv, 0.0..=1.0).text("Громкость мониторинга 🔊"))
                            .changed()
                        {
                            if self.config.output_devices.len() > 1 {
                                self.config.output_devices[1].volume = mv;
                            } else {
                                self.config.output_devices.push(crate::config::OutputDeviceConfig {
                                    name: None,
                                    volume: mv,
                                    enabled: true,
                                });
                            }
                            // Применяем сразу к играющим звукам (устройство monitor существует, так как selected_monitor_device > 0)
                            self.audio.borrow_mut().set_device_volume("monitor", mv).ok();
                            self.dirty = true;
                        }
                    }
                });

        ui.add_space(4.0);

        // --- Мастер-громкость ---
        ui.collapsing("🔊 Громкость", |ui| {
            let mut vol = self.config.master_volume;
            if ui
                .add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Мастер-громкость"))
                .changed()
            {
                self.config.master_volume = vol;
                self.dirty = true;
                // Применяем немедленно к играющим звукам.
                self.audio.borrow_mut().set_master_volume(vol);
            }
        });

        ui.add_space(4.0);

        // --- Поведение ---
        ui.collapsing("⚙ Поведение", |ui| {
            let mut stop_on_replay = self.config.stop_on_replay;
            ui.checkbox(&mut stop_on_replay, "Перезапуск при повторном нажатии");
            if stop_on_replay != self.config.stop_on_replay {
                self.config.stop_on_replay = stop_on_replay;
                self.dirty = true;
            }
        });

        ui.add_space(4.0);

        // --- Глобальная клавиша «Стоп всё» ---
        ui.collapsing("⏹ Глобальная клавиша «Остановить всё»", |ui| {
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
                "Нажмите… (Esc)".to_string()
            } else {
                stop_text
            };
            if ui.button(btn_text).clicked() && !capturing_stop {
                self.start_capture(CaptureTarget::StopAll);
            }
            if ui.small_button("✕ убрать").clicked() {
                if self.config.stop_all_hotkey.take().is_some() {
                    self.hotkeys.borrow_mut().unregister(HotkeyAction::StopAll);
                }
                self.dirty = true;
            }
        });

        ui.add_space(4.0);

        // --- PTT-имитация микрофона ---
        ui.collapsing("🎤 Имитация микрофона (Push-To-Talk)", |ui| {
            ui.label(
                "Укажите клавишу микрофона из вашей игры. Звукопад будет\n\
                 автоматически нажимать её во время воспроизведения звука,\n\
                 чтобы игра думала, что вы говорите. После окончания звука\n\
                 клавиша будет отпущена (с задержкой из поля ниже).",
            );
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
                "Нажмите… (Esc)".to_string()
            } else {
                ptt_text
            };
            if ui.button(btn_text).clicked() && !capturing_ptt {
                self.start_capture(CaptureTarget::Ptt);
            }
            if ui.small_button("✕ убрать").clicked() {
                self.config.ptt_hotkey = None;
                self.audio.borrow_mut().set_ptt_hotkey(None);
                self.dirty = true;
            }
            ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Задержка перед отпусканием (мс):");
                    let resp = ui.add(egui::DragValue::new(&mut self.config.ptt_release_delay_ms).range(0..=5000));
                    if resp.changed() {
                        self.dirty = true;
                        self.audio.borrow_mut().set_ptt_release_delay(self.config.ptt_release_delay_ms);
                    }
                });
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // --- Кнопки ---
        ui.horizontal(|ui| {
            if ui.button("➕ Добавить звук").clicked() {
                self.edit = Some(PendingEdit {
                    index: self.config.sounds.len(),
                    name: String::new(),
                    path: String::new(),
                    volume: 1.0,
                });
            }
            if ui.button("⏹ Остановить всё").clicked() {
                self.audio.borrow_mut().stop_all();
                self.status = "Всё остановлено".into();
            }
        });

        ui.add_space(8.0);

        // --- Подсказка ---
        self.draw_help(ui);
    }

    /// Центральная панель: список звуков.
    fn draw_sounds_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔊 Звукопад");
        ui.label("Звуковая панель — нажмите ▶ или используйте горячие клавиши");
        ui.add_space(8.0);

        if self.config.sounds.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label("Список пуст.");
                ui.label("Нажмите «➕ Добавить звук» в панели справа.");
            });
            return;
        }

        let available_width = ui.available_width();
        let min_button_w = 220.0;
        let cols = ((available_width / min_button_w).ceil() as usize).max(1);

        egui::Grid::new("sound_grid")
            .num_columns(cols)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                let mut to_play: Option<usize> = None;
                let mut to_stop: Option<usize> = None;
                let mut to_capture: Option<usize> = None;
                let mut to_remove: Option<usize> = None;
                let mut to_edit: Option<usize> = None;
                let mut to_clear_hotkey: Option<usize> = None;
                let mut volume_changes: Vec<(usize, f32)> = Vec::new();

                let playing_ids: Vec<u32> = self.audio.borrow().playing_names().into_iter().map(|(id, _)| id).collect();

                for (i, entry) in self.config.sounds.iter().enumerate() {
                    let playing = playing_ids.contains(&entry.id);
                    let hotkey_text = entry
                        .hotkey
                        .as_ref()
                        .map(hotkeys::hotkey_display)
                        .unwrap_or_else(|| "—".to_string());

                    ui.vertical(|ui| {
                        ui.set_min_width(210.0);
                        ui.horizontal(|ui| {
                            let label = if playing {
                                format!("⏸ {}", entry.name)
                            } else {
                                format!("▶ {}", entry.name)
                            };
                            if ui.button(label).clicked() {
                                if playing {
                                    to_stop = Some(i);
                                } else {
                                    to_play = Some(i);
                                }
                            }
                            if ui.button("✎").clicked() {
                                to_edit = Some(i);
                            }
                            if ui.button("🗑").clicked() {
                                to_remove = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Клавиша:");
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
                            if entry.hotkey.is_some() && ui.small_button("✕").clicked() {
                                to_clear_hotkey = Some(i);
                            }
                        });
                        let mut v = entry.volume;
                        let resp = ui.add(
                            egui::Slider::new(&mut v, 0.0..=1.0)
                                .show_value(false)
                                .text("громкость"),
                        );
                        if resp.changed() {
                            volume_changes.push((i, v));
                        }
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&entry.path)
                                    .small()
                                    .color(egui::Color32::GRAY),
                            )
                            .truncate(),
                        );
                    });
                    ui.end_row();
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
                        self.hotkeys.borrow_mut().unregister(HotkeyAction::Play(e.id));
                        e.hotkey = None;
                        self.dirty = true;
                    }
                }
                for (i, v) in volume_changes {
                    if let Some(e) = self.config.sounds.get_mut(i) {
                        e.volume = v;
                        // Применяем громкость немедленно к играющему звуку.
                        self.audio.borrow_mut().set_entry_volume(e.id, v);
                        self.dirty = true;
                    }
                }
            });
    }

    fn remove_entry(&mut self, index: usize) {
        if let Some(entry) = self.config.sounds.get(index) {
            self.hotkeys.borrow_mut().unregister(HotkeyAction::Play(entry.id));
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
                ui.label("Название:");
                ui.text_edit_singleline(&mut edit.name);

                ui.add_space(6.0);
                ui.label("Путь к звуковому файлу (mp3, wav, ogg, flac):");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut edit.path);
                    if ui.button("Обзор…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(
                                "Звук",
                                &["mp3", "wav", "ogg", "flac", "m4a", "aac"],
                            )
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

                ui.add_space(6.0);
                ui.label("Громкость:");
                ui.add(egui::Slider::new(&mut edit.volume, 0.0..=1.0));

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Сохранить").clicked() {
                        apply = true;
                    }
                    if ui.button("Отмена").clicked() {
                        cancel = true;
                    }
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
        ui.collapsing("📖 Как пользоваться Звукопадом", |ui| {
            ui.label("1. Добавьте звуки через «➕ Добавить звук».");
            ui.add_space(2.0);
            ui.label("2. Назначьте горячие клавиши — любая комбинация или просто одна клавиша (например Numpad1).");
            ui.add_space(2.0);
            ui.label(
                "3. Чтобы звук шёл в микрофон (Discord, OBS, игры):\n\
                 • Скачайте и установите VB-Audio Virtual Cable:\n",
            );
            ui.hyperlink_to(
                "    🌐 https://vb-audio.com/Cable/",
                "https://vb-audio.com/Cable/",
            );
            ui.add_space(2.0);
            ui.label(
                "   • В настройках Звукопада:\n\
                     — Основное устройство: выберите «CABLE Input (VB-Audio…)»\n\
                     — Устройство мониторинга: выберите ваши наушники (необязательно!)\n\
                 • В Discord / OBS / игре установите микрофон: «CABLE Output»",
            );
            ui.add_space(2.0);
            ui.label(
                "4. Для автоматического включения микрофона:\n\
                 • В разделе «Имитация микрофона» укажите клавишу,\n\
                   которая открывает голосовой чат в вашей игре.\n\
                 • Во время воспроизведения Звукопад автоматически\n\
                   нажмёт эту клавишу — игра подумает, что вы говорите.",
            );
            ui.add_space(2.0);
            ui.label(
                "5. Глобальная клавиша «Остановить всё» позволяет мгновенно\n\
                   остановить все играющие звуки одной кнопкой.",
            );
        });
    }
}


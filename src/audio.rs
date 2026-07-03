//! Аудио-движок: вывод на несколько устройств (основное + мониторинг),
//! воспроизведение звуков и интеграция с PTT (push-to-talk).
//!
//! Каждый звук создаёт по одному Sink на каждое активное устройство, чтобы
//! звук шёл одновременно и в кабель (для передачи), и в другое устройство
//! (для контроля/мониторинга). Громкость можно менять на лету.

use crate::config::HotkeyConfig;
use crate::ptt;
use rodio::cpal;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::{Duration, Instant};

/// Имя устройства вывода по умолчанию (для отображения в списке).
pub const DEFAULT_DEVICE_NAME: &str = "Системное устройство по умолчанию";

/// Имя пункта «нет устройства» (для мониторинга).
pub const NO_DEVICE_NAME: &str = "— нет (не использовать) —";

/// Описание доступного устройства вывода.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Активно играющий звук. Хранит sink-и для каждого устройства.
struct PlayingSound {
    /// Sink-и по device_id.
    sinks: HashMap<String, Sink>,
    /// Индивидуальная громкость (0.0..=1.0), без учёта мастер-громкости.
    own_volume: f32,
    name: String,
    #[allow(dead_code)]
    started_at: Instant,
}

/// Аудио-движок. Держит основной и мониторный потоки вывода.
pub struct AudioEngine {
    /// Map of device ID to (stream, handle, name, volume, enabled)
    devices: HashMap<String, (OutputStream, OutputStreamHandle, String, f32, bool)>,
    playing: HashMap<u32, PlayingSound>,

    ptt_hotkey: Option<HotkeyConfig>,
    ptt_held: bool,
    ptt_release_delay: Duration,
    ptt_release_at: Option<Instant>,

    master_volume: f32,
}

impl AudioEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            devices: HashMap::new(),
            playing: HashMap::new(),
            ptt_hotkey: None,
            ptt_held: false,
            ptt_release_delay: Duration::from_millis(300),
            ptt_release_at: None,
            master_volume: 1.0,
        };
        
        // Initialize default device if available
        if let Ok((stream, handle)) = build_stream(None) {
            engine.devices.insert(
                "default".to_string(),
                (stream, handle, "Системное устройство по умолчанию".to_string(), 1.0, true)
            );
        } else {
            log::error!("Не удалось инициализировать устройство по умолчанию");
        }
        
        engine
    }

    /// Перечисляет доступные устройства вывода.
    pub fn list_output_devices() -> Vec<DeviceInfo> {
        let mut devices = vec![DeviceInfo {
            name: DEFAULT_DEVICE_NAME.to_string(),
            is_default: true,
        }];
        match cpal::default_host().output_devices() {
            Ok(iter) => {
                for dev in iter {
                    let name = dev.name().unwrap_or_else(|_| "Без имени".to_string());
                    devices.push(DeviceInfo {
                        name,
                        is_default: false,
                    });
                }
            }
            Err(e) => {
                log::error!("Не удалось получить список устройств: {e}");
            }
        }
        devices
    }

    /// Добавляет или обновляет устройство вывода
    pub fn add_device(&mut self, device_id: &str, device_name: Option<&str>, volume: f32, enabled: bool) -> Result<(), String> {
        self.stop_all();
        
        // Remove existing device if present
        self.devices.remove(device_id);
        
        if !enabled {
            return Ok(());
        }
        
        let actual_name = device_name.unwrap_or(DEFAULT_DEVICE_NAME);
        let actual = if actual_name == DEFAULT_DEVICE_NAME {
            None
        } else {
            Some(actual_name)
        };
        
        let (stream, handle) = build_stream(actual)
            .map_err(|e| format!("Не удалось открыть устройство «{actual_name}»: {e}"))?;
            
        self.devices.insert(
            device_id.to_string(),
            (stream, handle, actual_name.to_string(), volume, true)
        );
        
        log::info!("Устройство добавлено/обновлено: {} (громкость {})", actual_name, volume);
        Ok(())
    }
    
    /// Удаляет устройство вывода
    pub fn remove_device(&mut self, device_id: &str) {
        self.stop_all();
        self.devices.remove(device_id);
        log::info!("Устройство удалено: {}", device_id);
    }
    
    /// Устанавливает громкость для устройства
    pub fn set_device_volume(&mut self, device_id: &str, volume: f32) -> Result<(), String> {
        if let Some((_, _, _, old_vol, _)) = self.devices.get_mut(device_id) {
            *old_vol = volume;
            for sound in self.playing.values_mut() {
                if let Some(sink) = sound.sinks.get_mut(device_id) {
                    sink.set_volume((sound.own_volume * volume * self.master_volume).clamp(0.0, 1.0));
                }
            }
            Ok(())
        } else {
            Err(format!("Устройство {} не найдено", device_id))
        }
    }

    pub fn set_ptt_release_delay(&mut self, delay_ms: u64) {
        self.ptt_release_delay = Duration::from_millis(delay_ms);
    }

    pub fn set_ptt_hotkey(&mut self, hk: Option<HotkeyConfig>) {
        if self.ptt_held {
            if let Some(old) = &self.ptt_hotkey {
                ptt::release_ptt(old);
            }
            self.ptt_held = false;
        }
        self.ptt_hotkey = hk;
    }

    /// Задаёт мастер-громкость и сразу применяет к играющим звукам.
    pub fn set_master_volume(&mut self, vol: f32) {
        self.master_volume = vol;
        self.apply_master_volume(vol);
    }

    /// Воспроизводит звук на всех активных устройствах.
    pub fn play(
        &mut self,
        entry_id: u32,
        path: &str,
        name: &str,
        volume: f32,
        master_volume: f32,
        stop_on_replay: bool,
    ) -> Result<bool, String> {
        if self.devices.is_empty() {
            return Err("Нет активных аудиоустройств".into());
        }
        if self.playing.contains_key(&entry_id) {
            if stop_on_replay {
                self.stop_internal(entry_id);
            } else {
                return Ok(false);
            }
        }

        let mut sinks: HashMap<String, Sink> = HashMap::new();
        
        let file = File::open(path)
            .map_err(|e| format!("Не удалось открыть «{path}»: {e}"))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Не удалось декодировать «{path}»: {e}"))?
            .buffered();

        for (device_id, (_, handle, _, device_vol, enabled)) in &self.devices {
            if !enabled {
                continue;
            }
            
            match Sink::try_new(handle) {
                Ok(sink) => {
                    sink.set_volume((volume * device_vol * master_volume).clamp(0.0, 1.0));
                    sink.append(source.clone());
                    sinks.insert(device_id.clone(), sink);
                }
                Err(e) => log::warn!("Не удалось создать sink для устройства {}: {e}", device_id),
            }
        }

        self.playing.insert(
            entry_id,
            PlayingSound {
                sinks,
                own_volume: volume,
                name: name.to_string(),
                started_at: Instant::now(),
            },
        );

        self.ensure_ptt_pressed();
        Ok(true)
    }

    pub fn stop(&mut self, entry_id: u32) {
        self.stop_internal(entry_id);
        self.maybe_release_ptt();
    }

    fn stop_internal(&mut self, entry_id: u32) {
        if let Some(p) = self.playing.remove(&entry_id) {
            for (_, sink) in p.sinks {
                sink.stop();
            }
        }
    }

    pub fn stop_all(&mut self) {
        let was_playing = !self.playing.is_empty();
        for (_, p) in self.playing.drain() {
            for (_, sink) in p.sinks {
                sink.stop();
            }
        }
        if was_playing {
            self.maybe_release_ptt();
        }
    }

    /// Обновляет громкость конкретного играющего звука (в реальном времени).
    pub fn set_entry_volume(&mut self, entry_id: u32, volume: f32) {
        if let Some(p) = self.playing.get_mut(&entry_id) {
            p.own_volume = volume;
            for (device_id, sink) in &mut p.sinks {
                let device_vol = self.devices.get(device_id).map(|d| d.3).unwrap_or(1.0);
                sink.set_volume((volume * device_vol * self.master_volume).clamp(0.0, 1.0));
            }
        }
    }

    pub fn playing_names(&self) -> Vec<(u32, String)> {
        self.playing
            .iter()
            .map(|(id, p)| (*id, p.name.clone()))
            .collect()
    }

    /// Применяет мастер-громкость к играющим звукам (все устройства).
    pub fn apply_master_volume(&mut self, master_volume: f32) {
        for p in self.playing.values() {
            for (device_id, sink) in &p.sinks {
                if let Some((_, _, _, device_vol, _)) = self.devices.get(device_id) {
                    sink.set_volume((p.own_volume * device_vol * master_volume).clamp(0.0, 1.0));
                }
            }
        }
    }

    pub fn cleanup_finished(&mut self) {
        let finished: Vec<u32> = self
            .playing
            .iter()
            .filter(|(_, p)| p.sinks.values().all(|s| s.empty()))
            .map(|(id, _)| *id)
            .collect();
        for id in finished {
            self.stop_internal(id);
        }
        if self.playing.is_empty() {
            self.maybe_release_ptt();
        }
    }

    // ------- PTT -------

    fn ensure_ptt_pressed(&mut self) {
        if self.ptt_held {
            return;
        }
        if let Some(hk) = &self.ptt_hotkey {
            if ptt::press_ptt(hk) {
                self.ptt_held = true;
                log::debug!("PTT нажата");
            }
        }
    }

    fn maybe_release_ptt(&mut self) {
        if !self.playing.is_empty() {
            self.ptt_release_at = None;
            return;
        }
        if !self.ptt_held {
            self.ptt_release_at = None;
            return;
        }
        let Some(hk) = &self.ptt_hotkey else {
            self.ptt_release_at = None;
            return;
        };

        if let Some(release_at) = self.ptt_release_at {
            if Instant::now() >= release_at {
                ptt::release_ptt(hk);
                self.ptt_held = false;
                self.ptt_release_at = None;
                log::debug!("PTT отпущена (с задержкой)");
            }
        } else {
            self.ptt_release_at = Some(Instant::now() + self.ptt_release_delay);
            log::debug!("PTT отпустится через {:?}", self.ptt_release_delay);
        }
    }

}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}



/// Создаёт OutputStream + handle для устройства по имени. None = по умолчанию.
fn build_stream(device_name: Option<&str>) -> Result<(OutputStream, OutputStreamHandle), String> {
    match device_name {
        None => OutputStream::try_default()
            .map_err(|e| format!("Устр-во по умолчанию: {e}")),
        Some(name) => {
            let device = cpal::default_host()
                .output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| format!("Устройство «{name}» не найдено"))?;
            OutputStream::try_from_device(&device)
                .map_err(|e| format!("Открытие устройства «{name}»: {e}"))
        }
    }
}

//! Конфигурация приложения: список звуков, устройства вывода,
//! мастер-громкость и глобальные горячие клавиши (стоп/PTT).
//! Хранится в JSON-файле в пользовательской папке.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Описание горячей клавиши в конфиге.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Модификаторы: "ctrl", "shift", "alt", "super" — через запятую.
    /// Может быть пустым (например, только клавиша Numpad1).
    #[serde(default)]
    pub modifiers: Vec<String>,
    /// Имя клавиши, например "Digit1", "KeyF", "F5", "Numpad1".
    pub code: String,
}

impl HotkeyConfig {
    pub fn new(modifiers: Vec<String>, code: String) -> Self {
        Self { modifiers, code }
    }
}

/// Одна запись звуковой панели.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundEntry {
    /// Уникальный идентификатор записи.
    pub id: u32,
    /// Отображаемое имя.
    pub name: String,
    /// Путь к звуковому файлу.
    pub path: String,
    /// Громкость конкретного звука от 0.0 до 1.0.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Назначенная горячая клавиша (необязательно).
    pub hotkey: Option<HotkeyConfig>,
}

fn default_volume() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

/// Настройка одного устройства вывода.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDeviceConfig {
    /// Имя устройства (None = системное по умолчанию).
    #[serde(default)]
    pub name: Option<String>,
    /// Громкость этого устройства (0.0..=1.0).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Включено ли устройство.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for OutputDeviceConfig {
    fn default() -> Self {
        Self {
            name: None,
            volume: 1.0,
            enabled: true,
        }
    }
}

/// Корневая конфигурация приложения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Записи звуковой панели.
    #[serde(default)]
    pub sounds: Vec<SoundEntry>,

    /// Список устройств вывода. Каждое устройство получает звук параллельно.
    /// Можно добавить несколько: VB-Cable, наушники, колонки и т.д.
    #[serde(default)]
    pub output_devices: Vec<OutputDeviceConfig>,

    /// Мастер-громкость (0.0..=1.0) — множитель на все устройства.
    #[serde(default = "default_volume")]
    pub master_volume: f32,

    /// Останавливать ли все звуки при повторном нажатии той же горячей клавиши.
    #[serde(default = "default_true")]
    pub stop_on_replay: bool,

    /// Глобальная горячая клавиша «Остановить всё».
    #[serde(default)]
    pub stop_all_hotkey: Option<HotkeyConfig>,

    /// Глобальная горячая клавиша «нажать микрофон» (PTT) — клавиша, которая будет
    /// удерживаться нажатой в системе, пока играет звук. Например, "V" или "X"
    /// (ту, что в игре отвечает за голосовой чат).
    #[serde(default)]
    pub ptt_hotkey: Option<HotkeyConfig>,

    /// Задержка (в миллисекундах) перед отпусканием PTT-клавиши после остановки звука,
    /// чтобы «хвост» звука точно ушёл в игру.
    #[serde(default = "ptt_release_delay_default")]
    pub ptt_release_delay_ms: u64,
}

fn ptt_release_delay_default() -> u64 {
    300
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sounds: Vec::new(),
            output_devices: Vec::new(),
            master_volume: 1.0,
            stop_on_replay: true,
            stop_all_hotkey: None,
            ptt_hotkey: None,
            ptt_release_delay_ms: 300,
        }
    }
}

impl Config {
    /// Возвращает путь к файлу конфигурации (%APPDATA%/zvukopad/config.json).
    pub fn path() -> Option<PathBuf> {
        let dir = dirs::config_dir()?;
        Some(dir.join("zvukopad").join("config.json"))
    }

    /// Загружает конфигурацию. При отсутствии/повреждении возвращает значение по умолчанию.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            log::warn!("Не удалось определить папку конфигурации.");
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::error!("Не удалось разобрать конфигурацию {}: {e}", path.display());
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("Конфигурация не найдена, используется значение по умолчанию.");
                Self::default()
            }
            Err(e) => {
                log::error!("Ошибка чтения конфигурации {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// Сохраняет конфигурацию в файл.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Папка конфигурации недоступна",
            ));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(&path, text)?;
        log::info!("Конфигурация сохранена: {}", path.display());
        Ok(())
    }

    /// Возвращает следующий свободный идентификатор для новой записи.
    pub fn next_id(&self) -> u32 {
        self.sounds.iter().map(|s| s.id).max().unwrap_or(0) + 1
    }
}
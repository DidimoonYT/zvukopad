//! Глобальные горячие клавиши.
//!
//! Обёртка над global-hotkey 0.6. `HotKey::id()` вычисляется автоматически
//! из модификаторов и клавиши, поэтому сопоставление нажатой клавиши с действием
//! ведём через отдельные карты.
//!
//! Поддерживаются два типа действий:
//!  - Воспроизвести запись (keyed by entry id)
//!  - Специальные: STOP_ALL и PTT (push-to-talk) — с зарезервированными маркерами.

use crate::config::HotkeyConfig;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};

/// Действие, привязанное к горячей клавише.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    /// Воспроизвести запись с указанным id.
    Play(u32),
    /// Остановить все звуки.
    StopAll,
    /// Нажать/удерживать PTT-клавишу (используется отдельно для логики,
    /// здесь — для полноты; реальная имитация клавиши делается в audio.rs).
    #[allow(dead_code)]
    Ptt,
}

/// Менеджер глобальных горячих клавиш.
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    /// id горячей клавиши -> действие. Карта общая с фоновым приёмником:
    /// события приходят в отдельном потоке и должны знать, что нажимать.
    action_map: Arc<Mutex<HashMap<u32, HotkeyAction>>>,
    /// Действие -> зарегистрированный HotKey (для unregister).
    registered: HashMap<HotkeyAction, HotKey>,
}

/// Фоновый приёмник нажатий.
///
/// `WM_HOTKEY` приходит на скрытое окно, созданное на потоке event loop.
/// Если разбирать события только в `update()`, при невидимом/свёрнутом окне
/// (и особенно когда eframe перестаёт перерисовываться) нажатия могут
/// «зависнуть» в очереди. Поэтому события читает отдельный поток, а окно
/// будится через `request_repaint`.
pub struct HotkeyBridge {
    rx: mpsc::Receiver<HotkeyAction>,
}

impl HotkeyBridge {
    /// Запускает поток-приёмник. `wake` вызывается после каждого нажатия,
    /// чтобы UI проснулся и обработал действие.
    pub fn spawn(
        action_map: Arc<Mutex<HashMap<u32, HotkeyAction>>>,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("zvukopad-hotkeys".into())
            .spawn(move || {
                log::info!("Поток горячих клавиш запущен");
                while let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                    if !matches!(event.state, HotKeyState::Pressed) {
                        continue;
                    }
                    log::debug!("Событие хоткея id={}", event.id);
                    let action = action_map
                        .lock()
                        .ok()
                        .and_then(|map| map.get(&event.id).copied());
                    if let Some(action) = action {
                        if tx.send(action).is_err() {
                            break;
                        }
                        wake();
                    }
                }
            })
            .expect("не удалось запустить поток горячих клавиш");
        Self { rx }
    }

    /// Забирает накопившиеся действия (неблокирующе).
    pub fn drain(&self) -> Vec<HotkeyAction> {
        self.rx.try_iter().collect()
    }
}

impl HotkeyManager {
    /// Создаёт менеджер (в том же потоке, что и цикл событий).
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        Ok(Self {
            manager,
            action_map: Arc::new(Mutex::new(HashMap::new())),
            registered: HashMap::new(),
        })
    }

    /// Общая карта id -> действие для фонового приёмника.
    pub fn action_map(&self) -> Arc<Mutex<HashMap<u32, HotkeyAction>>> {
        Arc::clone(&self.action_map)
    }

    /// Регистрирует горячую клавишу для действия. Если у действия уже была клавиша —
    /// снимает прежнюю. Возвращает ошибку, если комбинация занята другим действием.
    ///
    /// Прежняя клавиша снимается только после того, как новая прошла проверку:
    /// если регистрация не удалась (комбинацию занял другой процесс), рабочая
    /// настройка остаётся действующей и не расходится с конфигом.
    pub fn register(&mut self, action: HotkeyAction, hk: &HotkeyConfig) -> Result<(), String> {
        if hk.code.is_empty() {
            return Err("Код клавиши не может быть пустым".into());
        }

        let hotkey = build_hotkey(hk)?;
        let id = hotkey.id();
        {
            let map = self
                .action_map
                .lock()
                .map_err(|_| "Карта горячих клавиш заблокирована")?;
            if let Some(other) = map.get(&id) {
                if *other != action {
                    return Err(format!("Комбинация занята другим действием: {other:?}"));
                }
            }
        }

        let previous = self.registered.get(&action).copied();
        self.unregister(action);

        match self.manager.register(hotkey) {
            Ok(()) => {
                log::debug!("Хоткей {} зарегистрирован, id={id}", hk.code);
                self.map_insert(id, action);
                self.registered.insert(action, hotkey);
                Ok(())
            }
            Err(e) => {
                // Возвращаем прежнюю клавишу, чтобы действие не осталось
                // без хоткея из-за неудачной попытки.
                if let Some(prev) = previous {
                    if self.manager.register(prev).is_ok() {
                        self.map_insert(prev.id(), action);
                        self.registered.insert(action, prev);
                    }
                }
                Err(format!("Не удалось зарегистрировать клавишу: {e}"))
            }
        }
    }

    fn map_insert(&self, id: u32, action: HotkeyAction) {
        if let Ok(mut map) = self.action_map.lock() {
            map.insert(id, action);
        }
    }

    fn map_remove(&self, id: u32) {
        if let Ok(mut map) = self.action_map.lock() {
            map.remove(&id);
        }
    }

    fn map_clear(&self) {
        if let Ok(mut map) = self.action_map.lock() {
            map.clear();
        }
    }

    /// Снимает регистрацию горячей клавиши действия.
    pub fn unregister(&mut self, action: HotkeyAction) {
        if let Some(hk) = self.registered.remove(&action) {
            self.map_remove(hk.id());
            let _ = self.manager.unregister(hk);
        }
    }

    /// Снимает все регистрации.
    pub fn unregister_all(&mut self) {
        for (_, hk) in self.registered.drain() {
            let _ = self.manager.unregister(hk);
        }
        self.map_clear();
    }
}

/// Строит HotKey из строкового конфига. Модификаторы НЕ обязательны —
/// можно зарегистрировать просто клавишу (например, Numpad1).
fn build_hotkey(hk: &HotkeyConfig) -> Result<HotKey, String> {
    let mut mods = Modifiers::empty();
    for m in &hk.modifiers {
        match m.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "super" | "win" | "meta" | "cmd" | "command" => mods |= Modifiers::SUPER,
            other => return Err(format!("Неизвестный модификатор: {other}")),
        }
    }
    let code =
        code_from_str(&hk.code).ok_or_else(|| format!("Неизвестная клавиша: {}", hk.code))?;
    Ok(HotKey::new(
        if mods.is_empty() { None } else { Some(mods) },
        code,
    ))
}

/// Человекочитаемое представление горячей клавиши.
pub fn hotkey_display(hk: &HotkeyConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    for m in &hk.modifiers {
        let label = match m.to_lowercase().as_str() {
            "ctrl" | "control" => "Ctrl",
            "shift" => "Shift",
            "alt" | "option" => "Alt",
            "super" | "win" | "meta" | "cmd" | "command" => "Win",
            _ => m.as_str(),
        };
        parts.push(label.to_string());
    }
    parts.push(code_display(&hk.code));
    parts.join(" + ")
}

/// Краткое наглядное представление кода клавиши.
fn code_display(code: &str) -> String {
    let upper = code.to_uppercase();
    match upper.as_str() {
        s if s.starts_with("KEY") && s.len() > 3 => s[3..].to_string(),
        s if s.starts_with("DIGIT") => s[5..].to_string(),
        s if s.starts_with("NUMPAD") => format!("Num{}", &s[6..]),
        "ARROWUP" => "↑".into(),
        "ARROWDOWN" => "↓".into(),
        "ARROWLEFT" => "←".into(),
        "ARROWRIGHT" => "→".into(),
        "SPACE" => "Пробел".into(),
        "ENTER" => "Enter".into(),
        "ESCAPE" => "Esc".into(),
        "BACKSPACE" => "Backspace".into(),
        "INSERT" => "Insert".into(),
        "DELETE" => "Delete".into(),
        "HOME" => "Home".into(),
        "END" => "End".into(),
        "PAGEUP" => "PageUp".into(),
        "PAGEDOWN" => "PageDown".into(),
        _ => upper,
    }
}

/// Парсит код клавиши из строкового представления в `Code`.
pub fn code_from_str(code: &str) -> Option<Code> {
    use Code::*;
    let key = code.trim();
    match key.to_uppercase().as_str() {
        "BACKQUOTE" | "`" => Some(Backquote),
        "BACKSLASH" | "\\" => Some(Backslash),
        "BRACKETLEFT" | "[" => Some(BracketLeft),
        "BRACKETRIGHT" | "]" => Some(BracketRight),
        "PAUSE" | "PAUSEBREAK" => Some(Pause),
        "COMMA" | "," => Some(Comma),
        "DIGIT0" | "0" => Some(Digit0),
        "DIGIT1" | "1" => Some(Digit1),
        "DIGIT2" | "2" => Some(Digit2),
        "DIGIT3" | "3" => Some(Digit3),
        "DIGIT4" | "4" => Some(Digit4),
        "DIGIT5" | "5" => Some(Digit5),
        "DIGIT6" | "6" => Some(Digit6),
        "DIGIT7" | "7" => Some(Digit7),
        "DIGIT8" | "8" => Some(Digit8),
        "DIGIT9" | "9" => Some(Digit9),
        "EQUAL" | "=" => Some(Equal),
        "KEYA" | "A" => Some(KeyA),
        "KEYB" | "B" => Some(KeyB),
        "KEYC" | "C" => Some(KeyC),
        "KEYD" | "D" => Some(KeyD),
        "KEYE" | "E" => Some(KeyE),
        "KEYF" | "F" => Some(KeyF),
        "KEYG" | "G" => Some(KeyG),
        "KEYH" | "H" => Some(KeyH),
        "KEYI" | "I" => Some(KeyI),
        "KEYJ" | "J" => Some(KeyJ),
        "KEYK" | "K" => Some(KeyK),
        "KEYL" | "L" => Some(KeyL),
        "KEYM" | "M" => Some(KeyM),
        "KEYN" | "N" => Some(KeyN),
        "KEYO" | "O" => Some(KeyO),
        "KEYP" | "P" => Some(KeyP),
        "KEYQ" | "Q" => Some(KeyQ),
        "KEYR" | "R" => Some(KeyR),
        "KEYS" | "S" => Some(KeyS),
        "KEYT" | "T" => Some(KeyT),
        "KEYU" | "U" => Some(KeyU),
        "KEYV" | "V" => Some(KeyV),
        "KEYW" | "W" => Some(KeyW),
        "KEYX" | "X" => Some(KeyX),
        "KEYY" | "Y" => Some(KeyY),
        "KEYZ" | "Z" => Some(KeyZ),
        "MINUS" | "-" => Some(Minus),
        "PERIOD" | "." => Some(Period),
        "QUOTE" | "'" => Some(Quote),
        "SEMICOLON" | ";" => Some(Semicolon),
        "SLASH" | "/" => Some(Slash),
        "BACKSPACE" => Some(Backspace),
        "CAPSLOCK" => Some(CapsLock),
        "ENTER" => Some(Enter),
        "SPACE" => Some(Space),
        "TAB" => Some(Tab),
        "DELETE" => Some(Delete),
        "END" => Some(End),
        "HOME" => Some(Home),
        "INSERT" => Some(Insert),
        "PAGEDOWN" => Some(PageDown),
        "PAGEUP" => Some(PageUp),
        "PRINTSCREEN" => Some(PrintScreen),
        "SCROLLLOCK" => Some(ScrollLock),
        "ARROWDOWN" | "DOWN" => Some(ArrowDown),
        "ARROWLEFT" | "LEFT" => Some(ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Some(ArrowRight),
        "ARROWUP" | "UP" => Some(ArrowUp),
        "NUMPAD0" | "NUM0" => Some(Numpad0),
        "NUMPAD1" | "NUM1" => Some(Numpad1),
        "NUMPAD2" | "NUM2" => Some(Numpad2),
        "NUMPAD3" | "NUM3" => Some(Numpad3),
        "NUMPAD4" | "NUM4" => Some(Numpad4),
        "NUMPAD5" | "NUM5" => Some(Numpad5),
        "NUMPAD6" | "NUM6" => Some(Numpad6),
        "NUMPAD7" | "NUM7" => Some(Numpad7),
        "NUMPAD8" | "NUM8" => Some(Numpad8),
        "NUMPAD9" | "NUM9" => Some(Numpad9),
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => Some(NumpadAdd),
        "NUMPADDECIMAL" | "NUMDECIMAL" => Some(NumpadDecimal),
        "NUMPADDIVIDE" | "NUMDIVIDE" => Some(NumpadDivide),
        "NUMPADENTER" | "NUMENTER" => Some(NumpadEnter),
        "NUMPADEQUAL" | "NUMEQUAL" => Some(NumpadEqual),
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => Some(NumpadMultiply),
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => Some(NumpadSubtract),
        "ESCAPE" | "ESC" => Some(Escape),
        "F1" => Some(F1),
        "F2" => Some(F2),
        "F3" => Some(F3),
        "F4" => Some(F4),
        "F5" => Some(F5),
        "F6" => Some(F6),
        "F7" => Some(F7),
        "F8" => Some(F8),
        "F9" => Some(F9),
        "F10" => Some(F10),
        "F11" => Some(F11),
        "F12" => Some(F12),
        _ => None,
    }
}

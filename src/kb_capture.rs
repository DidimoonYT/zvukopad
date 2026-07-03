//! Захват нажатой клавиши напрямую через WinAPI.
//!
//! egui/egui-winit схлопывают Numpad и верхний ряд цифр в одну логическую
//! клавишу (Key::Num1), поэтому нельзя отличить Numpad1 от Digit1.
//! Чтобы корректно различать клавиши (включая Numpad), при захвате горячей
//! клавиши опрашиваем состояние всех VK-кодов через GetAsyncKeyState.
//!
//! Возвращает (имя_для_конфига, модификаторы). Имя соответствует формату,
//! который понимают hotkeys::code_from_str и ptt::vk_for_code.

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
}

/// Бит «клавиша нажата в данный момент» в результате GetAsyncKeyState.
const KEY_DOWN: i32 = 0x8000;

/// Результат захвата: основная клавиша + список модификаторов.
pub struct CapturedKey {
    /// Имя клавиши в формате конфига (например "Numpad1", "Digit5", "KeyF", "F5").
    pub code: String,
    /// Модификаторы: "ctrl", "shift", "alt", "super".
    pub modifiers: Vec<String>,
}

/// Описание VK-кода для таблицы опроса.
struct VkEntry {
    vk: i32,
    /// Имя в формате конфига.
    name: &'static str,
    /// Является ли эта клавиша модификатором (не основная).
    is_modifier: bool,
}

/// Полная таблица VK-кодов с именами в формате конфига.
/// Порядок важен: Numpad идёт ОТДЕЛЬНО от верхнего ряда цифр.
#[cfg(target_os = "windows")]
fn vk_table() -> Vec<VkEntry> {
    use VkEntry as E;
    vec![
        // Модификаторы
        E { vk: 0xA0, name: "shift", is_modifier: true },  // LSHIFT
        E { vk: 0xA1, name: "shift", is_modifier: true },  // RSHIFT
        E { vk: 0xA2, name: "ctrl", is_modifier: true },   // LCONTROL
        E { vk: 0xA3, name: "ctrl", is_modifier: true },   // RCONTROL
        E { vk: 0xA4, name: "alt", is_modifier: true },    // LMENU
        E { vk: 0xA5, name: "alt", is_modifier: true },    // RMENU
        E { vk: 0x5B, name: "super", is_modifier: true },  // LWIN
        E { vk: 0x5C, name: "super", is_modifier: true },  // RWIN
        // Буквы A..Z (VK совпадают с ASCII заглавными)
        E { vk: 0x41, name: "KeyA", is_modifier: false },
        E { vk: 0x42, name: "KeyB", is_modifier: false },
        E { vk: 0x43, name: "KeyC", is_modifier: false },
        E { vk: 0x44, name: "KeyD", is_modifier: false },
        E { vk: 0x45, name: "KeyE", is_modifier: false },
        E { vk: 0x46, name: "KeyF", is_modifier: false },
        E { vk: 0x47, name: "KeyG", is_modifier: false },
        E { vk: 0x48, name: "KeyH", is_modifier: false },
        E { vk: 0x49, name: "KeyI", is_modifier: false },
        E { vk: 0x4A, name: "KeyJ", is_modifier: false },
        E { vk: 0x4B, name: "KeyK", is_modifier: false },
        E { vk: 0x4C, name: "KeyL", is_modifier: false },
        E { vk: 0x4D, name: "KeyM", is_modifier: false },
        E { vk: 0x4E, name: "KeyN", is_modifier: false },
        E { vk: 0x4F, name: "KeyO", is_modifier: false },
        E { vk: 0x50, name: "KeyP", is_modifier: false },
        E { vk: 0x51, name: "KeyQ", is_modifier: false },
        E { vk: 0x52, name: "KeyR", is_modifier: false },
        E { vk: 0x53, name: "KeyS", is_modifier: false },
        E { vk: 0x54, name: "KeyT", is_modifier: false },
        E { vk: 0x55, name: "KeyU", is_modifier: false },
        E { vk: 0x56, name: "KeyV", is_modifier: false },
        E { vk: 0x57, name: "KeyW", is_modifier: false },
        E { vk: 0x58, name: "KeyX", is_modifier: false },
        E { vk: 0x59, name: "KeyY", is_modifier: false },
        E { vk: 0x5A, name: "KeyZ", is_modifier: false },
        // Верхний ряд цифр 0..9
        E { vk: 0x30, name: "Digit0", is_modifier: false },
        E { vk: 0x31, name: "Digit1", is_modifier: false },
        E { vk: 0x32, name: "Digit2", is_modifier: false },
        E { vk: 0x33, name: "Digit3", is_modifier: false },
        E { vk: 0x34, name: "Digit4", is_modifier: false },
        E { vk: 0x35, name: "Digit5", is_modifier: false },
        E { vk: 0x36, name: "Digit6", is_modifier: false },
        E { vk: 0x37, name: "Digit7", is_modifier: false },
        E { vk: 0x38, name: "Digit8", is_modifier: false },
        E { vk: 0x39, name: "Digit9", is_modifier: false },
        // Numpad 0..9 (отдельные VK!)
        E { vk: 0x60, name: "Numpad0", is_modifier: false },
        E { vk: 0x61, name: "Numpad1", is_modifier: false },
        E { vk: 0x62, name: "Numpad2", is_modifier: false },
        E { vk: 0x63, name: "Numpad3", is_modifier: false },
        E { vk: 0x64, name: "Numpad4", is_modifier: false },
        E { vk: 0x65, name: "Numpad5", is_modifier: false },
        E { vk: 0x66, name: "Numpad6", is_modifier: false },
        E { vk: 0x67, name: "Numpad7", is_modifier: false },
        E { vk: 0x68, name: "Numpad8", is_modifier: false },
        E { vk: 0x69, name: "Numpad9", is_modifier: false },
        // F-клавиши
        E { vk: 0x70, name: "F1", is_modifier: false },
        E { vk: 0x71, name: "F2", is_modifier: false },
        E { vk: 0x72, name: "F3", is_modifier: false },
        E { vk: 0x73, name: "F4", is_modifier: false },
        E { vk: 0x74, name: "F5", is_modifier: false },
        E { vk: 0x75, name: "F6", is_modifier: false },
        E { vk: 0x76, name: "F7", is_modifier: false },
        E { vk: 0x77, name: "F8", is_modifier: false },
        E { vk: 0x78, name: "F9", is_modifier: false },
        E { vk: 0x79, name: "F10", is_modifier: false },
        E { vk: 0x7A, name: "F11", is_modifier: false },
        E { vk: 0x7B, name: "F12", is_modifier: false },
        // Стрелки
        E { vk: 0x25, name: "ArrowLeft", is_modifier: false },
        E { vk: 0x26, name: "ArrowUp", is_modifier: false },
        E { vk: 0x27, name: "ArrowRight", is_modifier: false },
        E { vk: 0x28, name: "ArrowDown", is_modifier: false },
        // Навигация
        E { vk: 0x21, name: "PageUp", is_modifier: false },
        E { vk: 0x22, name: "PageDown", is_modifier: false },
        E { vk: 0x23, name: "End", is_modifier: false },
        E { vk: 0x24, name: "Home", is_modifier: false },
        E { vk: 0x2D, name: "Insert", is_modifier: false },
        E { vk: 0x2E, name: "Delete", is_modifier: false },
        // Numpad-операции
        E { vk: 0x6A, name: "NumpadMultiply", is_modifier: false },
        E { vk: 0x6B, name: "NumpadAdd", is_modifier: false },
        E { vk: 0x6D, name: "NumpadSubtract", is_modifier: false },
        E { vk: 0x6E, name: "NumpadDecimal", is_modifier: false },
        E { vk: 0x6F, name: "NumpadDivide", is_modifier: false },
        // Прочие
        E { vk: 0x20, name: "Space", is_modifier: false },
        E { vk: 0x0D, name: "Enter", is_modifier: false },
        E { vk: 0x09, name: "Tab", is_modifier: false },
        E { vk: 0x08, name: "Backspace", is_modifier: false },
        E { vk: 0x1B, name: "Escape", is_modifier: false },
        E { vk: 0x14, name: "CapsLock", is_modifier: false },
        // Знаки
        E { vk: 0xBA, name: "Semicolon", is_modifier: false },
        E { vk: 0xBB, name: "Equal", is_modifier: false },
        E { vk: 0xBC, name: "Comma", is_modifier: false },
        E { vk: 0xBD, name: "Minus", is_modifier: false },
        E { vk: 0xBE, name: "Period", is_modifier: false },
        E { vk: 0xBF, name: "Slash", is_modifier: false },
        E { vk: 0xC0, name: "Backquote", is_modifier: false },
        E { vk: 0xDB, name: "BracketLeft", is_modifier: false },
        E { vk: 0xDC, name: "Backslash", is_modifier: false },
        E { vk: 0xDD, name: "BracketRight", is_modifier: false },
        E { vk: 0xDE, name: "Quote", is_modifier: false },
    ]
}

/// Опрашивает состояние клавиш и возвращает нажатую комбинацию.
/// Логика:
///  - собираем все нажатые модификаторы;
///  - первую найденную НЕ-модификатор клавишу считаем основной;
///  - игнорируем чистые модификаторы (нужна хотя бы одна основная клавиша).
///
/// Возвращает None, если ни одна основная клавиша не нажата.
#[cfg(target_os = "windows")]
pub fn poll_pressed() -> Option<CapturedKey> {
    let table = vk_table();
    let mut modifiers: Vec<String> = Vec::new();
    let mut main_code: Option<String> = None;

    for entry in &table {
        if (unsafe { GetAsyncKeyState(entry.vk) } as i32) & KEY_DOWN != 0 {
            if entry.is_modifier {
                let name = entry.name.to_string();
                if !modifiers.contains(&name) {
                    modifiers.push(name);
                }
            } else if main_code.is_none() {
                main_code = Some(entry.name.to_string());
            }
        }
    }

    let code = main_code?;
    Some(CapturedKey { code, modifiers })
}

/// Проверяет, нажат ли Esc (для отмены захвата).
#[cfg(target_os = "windows")]
pub fn is_escape_pressed() -> bool {
    unsafe { GetAsyncKeyState(0x1B) as i32 & KEY_DOWN != 0 }
}

/// Проверяет, нажат ли Backspace (для очистки клавиши).
#[cfg(target_os = "windows")]
pub fn is_backspace_pressed() -> bool {
    unsafe { GetAsyncKeyState(0x08) as i32 & KEY_DOWN != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn poll_pressed() -> Option<CapturedKey> {
    None
}
#[cfg(not(target_os = "windows"))]
pub fn is_escape_pressed() -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn is_backspace_pressed() -> bool {
    false
}

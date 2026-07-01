//! Имитация нажатия клавиши микрофона (Push-To-Talk) через WinAPI SendInput.
//!
//! Пока играет звук, удерживаем указанную клавишу нажатой, чтобы игра/драйвер
//! сочли, что пользователь говорит в голосовой чат. Это НЕ完美的ное решение
//! (некоторые анти-читы блокируют синтетический ввод), но для большинства игр
//! и Discord работает.

#![cfg(target_os = "windows")]

use crate::config::HotkeyConfig;

/// WinAPI-обёртка для SendInput.
#[repr(C)]
struct InputUnion {
    ki: KeyboardInput,
}

#[repr(C)]
#[allow(non_snake_case)]
struct KeyboardInput {
    wVk: u16,
    wScan: u16,
    dwFlags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
struct Input {
    type_: u32,
    union: InputUnion,
}

unsafe extern "system" {
    fn SendInput(c_inputs: u32, inputs: *const Input, cb_size: i32) -> u32;
    fn MapVirtualKeyW(u_code: u32, u_map_type: u32) -> u32;
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_KEYUP: u32 = 0x0002;
/// Используем scan-код (надёжнее для игр, которые читают hardware-скан-коды).
const KEYEVENTF_SCANCODE: u32 = 0x0008;
const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
const MAPVK_VK_TO_VSC: u32 = 0;

/// Сопоставление имени клавиши (из нашего конфига) с Windows Virtual-Key кодом.
/// Возвращает (vk_code, is_extended), где is_extended нужен для Numpad/стрелок и т.п.
fn vk_for_code(code: &str) -> Option<(u16, bool)> {
    let upper = code.to_uppercase();
    match upper.as_str() {
        // Буквы
        s if s.starts_with("KEY") && s.len() == 4 => {
            let c = s.as_bytes()[3];
            if c.is_ascii_uppercase() {
                Some((c as u16, false)) // VK_A..VK_Z совпадают с ASCII
            } else {
                None
            }
        }
        // Цифры верхнего ряда
        s if s.starts_with("DIGIT") && s.len() == 6 => {
            let c = s.as_bytes()[5];
            if c.is_ascii_digit() {
                Some((c as u16, false))
            } else {
                None
            }
        }
        "0" => Some((b'0' as u16, false)),
        "1" => Some((b'1' as u16, false)),
        "2" => Some((b'2' as u16, false)),
        "3" => Some((b'3' as u16, false)),
        "4" => Some((b'4' as u16, false)),
        "5" => Some((b'5' as u16, false)),
        "6" => Some((b'6' as u16, false)),
        "7" => Some((b'7' as u16, false)),
        "8" => Some((b'8' as u16, false)),
        "9" => Some((b'9' as u16, false)),
        // F-клавиши
        "F1" => Some((0x70, false)),
        "F2" => Some((0x71, false)),
        "F3" => Some((0x72, false)),
        "F4" => Some((0x73, false)),
        "F5" => Some((0x74, false)),
        "F6" => Some((0x75, false)),
        "F7" => Some((0x76, false)),
        "F8" => Some((0x77, false)),
        "F9" => Some((0x78, false)),
        "F10" => Some((0x79, false)),
        "F11" => Some((0x7A, false)),
        "F12" => Some((0x7B, false)),
        // Numpad (extended)
        "NUMPAD0" | "NUM0" => Some((0x60, true)),
        "NUMPAD1" | "NUM1" => Some((0x61, true)),
        "NUMPAD2" | "NUM2" => Some((0x62, true)),
        "NUMPAD3" | "NUM3" => Some((0x63, true)),
        "NUMPAD4" | "NUM4" => Some((0x64, true)),
        "NUMPAD5" | "NUM5" => Some((0x65, true)),
        "NUMPAD6" | "NUM6" => Some((0x66, true)),
        "NUMPAD7" | "NUM7" => Some((0x67, true)),
        "NUMPAD8" | "NUM8" => Some((0x68, true)),
        "NUMPAD9" | "NUM9" => Some((0x69, true)),
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => Some((0x6B, true)),
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => Some((0x6D, true)),
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => Some((0x6A, true)),
        "NUMPADDIVIDE" | "NUMDIVIDE" => Some((0x6F, true)),
        "NUMPADDECIMAL" | "NUMDECIMAL" => Some((0x6E, true)),
        // Стрелки и блок навигации (extended)
        "ARROWUP" => Some((0x26, true)),
        "ARROWDOWN" => Some((0x28, true)),
        "ARROWLEFT" => Some((0x25, true)),
        "ARROWRIGHT" => Some((0x27, true)),
        "INSERT" => Some((0x2D, true)),
        "DELETE" => Some((0x2E, true)),
        "HOME" => Some((0x24, true)),
        "END" => Some((0x23, true)),
        "PAGEUP" => Some((0x21, true)),
        "PAGEDOWN" => Some((0x22, true)),
        // Прочее
        "SPACE" => Some((0x20, false)),
        "ENTER" => Some((0x0D, false)),
        "TAB" => Some((0x09, false)),
        "BACKSPACE" => Some((0x08, false)),
        "ESCAPE" | "ESC" => Some((0x1B, false)),
        "CAPSLOCK" => Some((0x14, false)),
        "MINUS" => Some((0xBD, false)),
        "EQUAL" => Some((0xBB, false)),
        "COMMA" => Some((0xBC, false)),
        "PERIOD" => Some((0xBE, false)),
        "SLASH" => Some((0xBF, false)),
        "SEMICOLON" => Some((0xBA, false)),
        "QUOTE" => Some((0xDE, false)),
        "BACKSLASH" => Some((0xDC, false)),
        "BRACKETLEFT" => Some((0xDB, false)),
        "BRACKETRIGHT" => Some((0xDD, false)),
        "BACKQUOTE" => Some((0xC0, false)),
        _ => None,
    }
}

/// Нажимает (и удерживает) клавишу PTT.
pub fn press_ptt(hk: &HotkeyConfig) -> bool {
    let Some((vk, extended)) = vk_for_code(&hk.code) else {
        log::warn!("PTT: неизвестная клавиша {}", hk.code);
        return false;
    };
    send_key(vk, extended, false)
}

/// Отпускает клавишу PTT.
pub fn release_ptt(hk: &HotkeyConfig) -> bool {
    let Some((vk, extended)) = vk_for_code(&hk.code) else {
        return false;
    };
    send_key(vk, extended, true)
}

/// Посылает событие нажатия/отпускания клавиши через SendInput.
/// Использует scan-код (KEYEVENTF_SCANCODE), что надёжнее для игр.
fn send_key(vk: u16, extended: bool, up: bool) -> bool {
    unsafe {
        let scan = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16;
        let mut flags = KEYEVENTF_SCANCODE;
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if up {
            flags |= KEYEVENTF_KEYUP;
        }
        let input = Input {
            type_: INPUT_KEYBOARD,
            union: InputUnion {
                ki: KeyboardInput {
                    wVk: 0, // по scan-коду, vk=0
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };
        let size = std::mem::size_of::<Input>() as i32;
        let sent = SendInput(1, &input, size);
        sent == 1
    }
}

#[cfg(not(target_os = "windows"))]
pub fn press_ptt(_hk: &HotkeyConfig) -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn release_ptt(_hk: &HotkeyConfig) -> bool {
    false
}

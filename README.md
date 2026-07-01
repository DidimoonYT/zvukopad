# 🔊 Звукопад — Бесплатный аналог Soundpad на Rust

**Звукопад** — это **бесплатный open-source аналог Soundpad** (саундборд / звуковая панель) для Windows с русским интерфейсом, поддержкой **любых горячих клавиш**, выводом звука **одновременно на два устройства** (виртуальный кабель + наушники), **глобальной кнопкой «Стоп всё»** и **имитацией нажатия клавиши микрофона (PTT)** в играх и Discord во время воспроизведения звуков.

[![GitHub release](https://img.shields.io/github/v/release/DidimoonYT/zvukopad?label=Latest%20Release&style=for-the-badge)](https://github.com/DidimoonYT/zvukopad/releases/latest)
[![GitHub downloads](https://img.shields.io/github/downloads/DidimoonYT/zvukopad/total?style=for-the-badge&logo=github&color=blue)](https://github.com/DidimoonYT/zvukopad/releases/latest)
[![GitHub last commit](https://img.shields.io/github/last-commit/DidimoonYT/zvukopad?style=for-the-badge)](https://github.com/DidimoonYT/zvukopad/commits/master)
[![License: MIT](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Windows-10%2F11-blue?style=for-the-badge&logo=windows)](https://github.com/DidimoonYT/zvukopad/releases/latest)

---

## 🚀 Быстрый старт — скачайте готовый .exe

### 📥 [![Download](https://img.shields.io/badge/Скачать_zvukopad.exe-4CAF50?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/DidimoonYT/zvukopad/releases/latest/download/zvukopad.exe)

**Прямые ссылки на релизы:**
- 🔗 **GitHub Releases:** https://github.com/DidimoonYT/zvukopad/releases/latest
- 🔗 **GitLab Releases:** https://gitlab.com/didimoonyt/zvukopad/-/releases

> После скачивания просто запустите `zvukopad.exe` — установка **не требуется** (портативный single-файл).

---

## 📦 Настройка виртуального кабеля (чтобы звук шёл «в микрофон» Discord/игр)

> Это **одноразовое действие** — нужно, чтобы Discord/OBS/игры слышали ваши звуки.

1. Скачайте и установите **VB-Audio Virtual Cable** (бесплатно):
   🌐 https://vb-audio.com/Cable/

2. В **Звукопаде** (раздел «🔊 Устройство вывода»):
   - **Основное устройство:** выберите **«CABLE Input (VB-Audio Virtual Cable)»**

3. В **Discord / OBS / игре** настройте микрофон:
   - Устройство ввода = **«CABLE Output»**

Теперь при нажатии горячей клавиши звук пойдёт собеседникам.

---

## 🎧 Мониторинг (слышать самому)

В разделе **«🎧 Устройство мониторинга»** выберите ваши наушники/колонки.

> ⚠️ **Это НЕОБЯЗАТЕЛЬНО.** Мониторинг нужен только вам, чтобы слышать, что играет. На передачу звука собеседникам он не влияет. Если не нужно — оставьте «— нет (не использовать) —».

У мониторинга есть **своя громкость** (отдельно от основной).

---

## 🎮 Имитация микрофона (PTT) для игр и Discord

В разделе **«🎤 Имитация микрофона»** укажите клавишу, которая в вашей игре/Дискорде открывает голосовой чат (например `V`, `X`, `CapsLock`, `Numpad0`).

- Во время воспроизведения звука Звукопад **автоматически нажмёт и удержит** эту клавишу — игра/Дискорд «подумает», что вы говорите, и пустит звук.
- После окончания звука клавиша будет **отпущена** с заданной задержкой (по умолчанию 300 мс — чтобы «хвост» точно ушёл).

> ⚠️ Имитация использует Windows `SendInput` со scan-кодами. Это работает в большинстве игр (CS2, Dota 2, Valorant, WoW и др.) и Discord, но некоторые анти-читы могут блокировать синтетический ввод.

---

## ⌨️ Назначение горячих клавиш

1. Добавьте звук через **➕ Добавить звук** → введите имя → «Обзор…» → выберите файл.
2. В списке звуков нажмите на кнопку клавиши и зажмите нужную комбинацию.
   - **Можно использовать ЛЮБУЮ клавишу**, даже без модификаторов (например `Numpad1`, `F1`, `Space`).
   - **Esc** — отменить назначение.
   - **Backspace** — убрать клавишу.
   - ✕ рядом с кнопкой — снять назначение.

---

## 🛠 Сборка из исходников (для разработчиков)

### Требования
- **Rust** (тестировалось на 1.75+)
- На Windows — нативный линкер: **MinGW-w64** (`dlltool`/`gcc` в PATH) или **MSVC Build Tools**

### Сборка
```bash
# GNU-тулчейн (по умолчанию, MinGW):
cargo build --release

# или MSVC (Visual Studio Build Tools):
rustup default stable-x86_64-pc-windows-msvc
cargo build --release
```

Готовый файл: `target/release/zvukopad.exe`

---

## 📁 Структура проекта

```
src/
├── main.rs       — точка входа, окно eframe (скрывает консоль в release)
├── app.rs        — интерфейс: список звуков, настройки, назначение клавиш, подсказка
├── audio.rs      — движок: два устройства вывода + PTT-интеграция (rodio)
├── ptt.rs        — имитация клавиши через WinAPI SendInput
├── hotkeys.rs    — глобальные горячие клавиши (global-hotkey), любая комбинация
├── kb_capture.rs — захват нажатия клавиш для назначения
└── config.rs     — конфигурация (JSON), автосохранение
```

---

## 🔧 Используемые библиотеки

| Библиотека | Назначение | Лицензия |
|------------|------------|----------|
| `eframe`/`egui` | Графический интерфейс (native) | MIT |
| `rodio` | Воспроизведение звука (cpal) | MIT |
| `global-hotkey` | Системные горячие клавиши | MIT |
| `rfd` | Нативный диалог выбора файла | MIT/Apache-2.0 |
| `serde`/`serde_json` | Сохранение конфигурации | MIT/Apache-2.0 |
| WinAPI `SendInput` | Имитация нажатия клавиши PTT | — |

---

## 📄 Лицензия

**MIT** — делайте что хотите: используйте в коммерции, продавайте, закрывайте код, меняйте лицензию. Только сохраняйте копию лицензии.

См. файл [LICENSE](LICENSE).

---

## 🤝 Поддержка проекта

Если проект полезен — поставьте ⭐ на GitHub/GitLab, это мотивирует развивать дальше!

- **Issues / Баги / Идеи:** [GitHub Issues](https://github.com/DidimoonYT/zvukopad/issues) / [GitLab Issues](https://gitlab.com/didimoonyt/zvukopad/-/issues)
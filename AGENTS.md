# Zvukopad — Rust Soundpad

## Язык
Всё общение на русском языке.

## Проект
**Звукопад** — бесплатный аналог Soundpad: звуковая панель с горячими клавишами, PTT-имитацией микрофона и выводом на 2 устройства.

- Репозиторий: https://github.com/DidimoonYT/zvukopad
- Лицензия: MIT

## Стек
- Rust 2021 edition
- egui/eframe 0.31
- rodio 0.20
- global-hotkey 0.6
- WinAPI SendInput (ptt.rs)
- embed-resource 2
- image 0.25
- serde + serde_json
- rfd 0.15
- dirs 6
- env_logger + log

## Структура
```
src/
  main.rs      — точка входа, иконка, eframe окно
  app.rs       — GUI на egui
  audio.rs     — воспроизведение, устройства вывода
  hotkeys.rs   — хоткеи
  ptt.rs       — PTT-имитация микрофона (SendInput)
  kb_capture.rs — захват клавиш
  config.rs    — конфигурация (JSON в %APPDATA%/zvukopad/)
  version.rs   — версия приложения
```

## Сборка
```bash
cargo build --release
```

## CI/CD
- GitHub Actions (`.github/workflows/release.yml`) — push тега `v*`
- GitLab CI (`.gitlab-ci.yml`) — push тега

## Правила
1. Перед изменением .rs файла убедись, что проект компилируется
2. `cargo clippy` — перед каждым коммитом
3. Версия в Cargo.toml — единственный источник истины
4. build.rs не менять без понимания логики кросс-компиляции

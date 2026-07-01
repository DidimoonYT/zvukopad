# 🔊 Звукопад — Бесплатный аналог Soundpad

**Звукопад** — звуковая панель для Windows с горячими клавишами, выводом на 2 устройства (VB-Cable + наушники), глобальной кнопкой «Стоп всё» и **PTT-имитацией микрофона** (авто-нажатие клавиши в играх/Дискорде).

> Скачайте, запустите — работает без установки (портативный `.exe`).

---

## 📥 Скачать

| Версия | Дата | Ссылка |
|--------|------|--------|
| **v1.0.6** | 2024-07-01 | [⬇️ zvukopad-v1.0.6.exe](https://github.com/DidimoonYT/zvukopad/releases/download/v1.0.6/zvukopad-v1.0.6.exe) |
| **v1.0.5** | 2024-07-01 | [⬇️ zvukopad-v1.0.5.exe](https://github.com/DidimoonYT/zvukopad/releases/download/v1.0.5/zvukopad-v1.0.5.exe) |
| **v1.0.4** | 2024-07-01 | [⬇️ zvukopad-v1.0.4.exe](https://github.com/DidimoonYT/zvukopad/releases/download/v1.0.4/zvukopad-v1.0.4.exe) |

> 💡 Все релизы: [GitHub Releases](https://github.com/DidimoonYT/zvukopad/releases) / [GitLab Releases](https://gitlab.com/didimoonyt/zvukopad/-/releases)

---

## 🚀 Быстрый старт

1. **Скачайте** `zvukopad-v1.0.6.exe` выше
2. **Установите VB-Cable** (один раз): https://vb-audio.com/Cable/
3. В Звукопаде: **Основное устройство → CABLE Input**
4. В Discord/игры: **Микрофон → CABLE Output**
5. Назначьте клавиши звукам — пользуйтесь

---

## ⌨️ Фичи в двух словах

- 🔀 **2 устройства одновременно** — основное (в Discord) + мониторинг (в наушники)
- 🎤 **PTT-имитация** — авто-зажимает вашу PTT-клавишу во время звука
- ⏹ **Глобальный «Стоп всё»** — одна клавиша останавливает все звуки
- ⌨️ **Любые хоткеи** — хоть `Numpad1`, хоть `Ctrl+Shift+F`, хоть просто `Space`
- 🔊 **Громкость** — мастер + на каждый звук + мониторинг отдельно
- 🪟 **Один `.exe`** — никакой установки, настройки в `%APPDATA%/zvukopad/`

---

## 🛠 Сборка

```bash
# MinGW (GNU)
cargo build --release

# MSVC (Visual Studio)
rustup default stable-x86_64-pc-windows-msvc
cargo build --release
```

Готово: `target/release/zvukopad.exe`

---

## 📄 Лицензия

MIT — делайте что хотите. См. [LICENSE](LICENSE).

---

⭐ Если полезно — поставьте звезду на [GitHub](https://github.com/DidimoonYT/zvukopad) или [GitLab](https://gitlab.com/didimoonyt/zvukopad)!
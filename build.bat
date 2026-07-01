@echo off
REM Скрипт сборки Звукопад

REM Получаем версию из Cargo.toml
for /f "tokens=2 delims==" %%a in ('findstr "^version" Cargo.toml') do set VERSION=%%a
set VERSION=%VERSION:"=%
set VERSION=%VERSION: =%

echo Версия: %VERSION%

echo Сборка...
cargo build --release

if errorlevel 1 (
    echo Ошибка сборки!
    exit /b 1
)

echo Копирование в корень проекта...
copy target\release\zvukopad.exe zvukopad.exe

echo.
echo Готово!
echo Создан: zvukopad.exe
echo Также доступен в: target\release\zvukopad.exe
@echo off
REM Скрипт сборки Звукопад с версией в имени файла

REM Получаем версию из Cargo.toml
for /f "tokens=2 delims==" %%a in ('findstr "^version" Cargo.toml') do set VERSION=%%a
REM Убираем кавычки и пробелы
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
copy target\release\zvukopad.exe zvukopad-v%VERSION%.exe
copy target\release\zvukopad.exe target\release\zvukopad-v%VERSION%.exe

echo.
echo Готово!
echo Создан: zvukopad-v%VERSION%.exe
echo Также доступен в: target\release\zvukopad-v%VERSION%.exe
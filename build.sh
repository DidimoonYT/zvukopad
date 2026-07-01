#!/usr/bin/env bash
set -e

# Скрипт сборки Звукопад с версией в имени файла

VERSION=$(grep '^version =' Cargo.toml | sed 's/.*= *"\([^"]*\)".*/\1/')
echo "Версия: $VERSION"

echo "Сборка..."
cargo build --release

echo "Копирование в корень проекта..."
cp target/release/zvukopad.exe "zvukopad-v$VERSION.exe"
cp target/release/zvukopad.exe "target/release/zvukopad-v$VERSION.exe"

echo ""
echo "Готово!"
echo "Создан: zvukopad-v$VERSION.exe"
echo "Также доступен в: target/release/zvukopad-v$VERSION.exe"
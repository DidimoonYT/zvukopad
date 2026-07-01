#!/usr/bin/env bash
set -e

# Скрипт сборки Звукопад

VERSION=$(grep '^version =' Cargo.toml | sed 's/.*= *"\([^"]*\)".*/\1/')
echo "Версия: $VERSION"

echo "Сборка..."
cargo build --release

echo "Копирование в корень проекта..."
cp target/release/zvukopad.exe zvukopad.exe

echo ""
echo "Готово!"
echo "Создан: zvukopad.exe"
echo "Также доступен в: target/release/zvukopad.exe"
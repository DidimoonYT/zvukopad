fn main() {
    // Встраиваем иконку в .exe как Windows resource
    // Это делает иконку видимой в проводнике Windows.
    //
    // Используем CARGO_CFG_TARGET_OS вместо #[cfg(target_os = "windows")],
    // потому что первый проверяет ЦЕЛЕВУЮ ОС (Windows) при кросс-компиляции,
    // а #[cfg(...)] проверяет ХОСТОВУЮ ОС (Linux в GitHub Actions / GitLab CI).
    //
    // Встраивание иконки не должно ломать сборку: если компилятор ресурсов
    // (windres) недоступен на хосте, просто пропускаем ресурс и продолжаем.
    // На CI переменная WINDOWS_RESOURCE_COMPILER указывает на mingw-w64 windres,
    // поэтому там иконка встраивается штатно.
    if std::env::var("CARGO_CFG_TARGET_OS")
        .map(|v| v == "windows")
        .unwrap_or(false)
    {
        let embedded = std::panic::catch_unwind(|| {
            embed_resource::compile("resource.rc", std::iter::empty::<&str>());
        });
        if embedded.is_err() {
            println!(
                "cargo:warning=windres не найден: иконка не встроена, сборка продолжается без ресурса."
            );
        }
    }

    // Переменная для post-build скриптов
    println!("cargo:rustc-env=ZVUKOPAD_VERSION={}", env!("CARGO_PKG_VERSION"));
}
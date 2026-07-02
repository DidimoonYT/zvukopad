fn main() {
    // Встраиваем иконку в .exe как Windows resource
    // Это делает иконку видимой в проводнике Windows
    //
    // Используем CARGO_CFG_TARGET_OS вместо #[cfg(target_os = "windows")],
    // потому что первый проверяет ЦЕЛЕВУЮ ОС (Windows) при кросс-компиляции,
    // а #[cfg(...)] проверяет ХОСТОВУЮ ОС (Linux в GitHub Actions / GitLab CI).
    if std::env::var("CARGO_CFG_TARGET_OS").map(|v| v == "windows").unwrap_or(false) {
        embed_resource::compile("resource.rc");
    }

    // Переменная для post-build скриптов
    println!("cargo:rustc-env=ZVUKOPAD_VERSION={}", env!("CARGO_PKG_VERSION"));
}

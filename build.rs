fn main() {
    // Встраиваем иконку в .exe как Windows resource
    // Это делает иконку видимой в проводнике Windows
    #[cfg(target_os = "windows")]
    {
        // Используем embed-resource crate который проще и надежнее
        embed_resource::compile("resource.rc");
    }

    // Устанавливаем переменную окружения для post-build копирования
    println!("cargo:rustc-env=ZVUKOPAD_VERSION={}", env!("CARGO_PKG_VERSION"));
}
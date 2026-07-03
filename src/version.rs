//! Версия приложения — единственный источник истины
//!
//! Используется везде: main.rs, app.rs, build.rs, CI/CD скрипты.
//! Обновляется автоматически из Cargo.toml при сборке.

/// Версия приложения (читается из Cargo.toml)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Полное название версии для отображения в UI
pub fn version_display() -> String {
    format!("Звукопад v{}", VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_format() {
        // Версия должна быть в формате X.Y.Z
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "Version must be X.Y.Z format");
        assert!(parts[0].parse::<u32>().is_ok(), "Major version must be numeric");
        assert!(parts[1].parse::<u32>().is_ok(), "Minor version must be numeric");
        assert!(parts[2].parse::<u32>().is_ok(), "Patch version must be numeric");
    }
}
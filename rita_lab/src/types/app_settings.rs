#[derive(PartialEq)]

/// Global app settings.
pub struct AppSettings {
    pub dark_mode: bool,
    pub sidebar_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: false,
            sidebar_enabled: true,
        }
    }
}

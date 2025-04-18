#[derive(PartialEq, Eq)]

// Determine which tab is currently open.
pub enum Tab {
    Lab,
    Debug,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Lab
    }
}

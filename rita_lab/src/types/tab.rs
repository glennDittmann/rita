use serde::{Deserialize, Serialize};

/// The tab which is currently open.
#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub enum Tab {
    Lab,
    Debug,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Lab
    }
}

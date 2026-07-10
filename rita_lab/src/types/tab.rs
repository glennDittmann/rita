use serde::{Deserialize, Serialize};

/// The tab which is currently open.
#[derive(Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Tab {
    #[default]
    Lab,
    Debug,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub(crate) name: String,
    pub(crate) count: u32,
}
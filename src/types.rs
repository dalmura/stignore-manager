use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::Add;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ItemGroup {
    pub id: String,
    pub name: String,
    pub size_kb: u64,
    pub items: Vec<ItemGroup>,
    pub leaf: bool,
}

impl PartialEq for ItemGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ItemGroup {}

impl Hash for ItemGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Add for ItemGroup {
    type Output = Self;

    // TODO: Update below to merging items and recalculate size_kb
    fn add(self, _other: Self) -> Self {
        Self {
            id: self.id,
            name: self.name,
            size_kb: 0,
            items: vec![],
            leaf: self.leaf,
        }
    }
}
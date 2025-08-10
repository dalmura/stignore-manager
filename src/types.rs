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

    fn add(self, other: Self) -> Self {
        use std::collections::HashMap;

        // Merge items by ID, using addition for duplicates
        let mut merged_items: HashMap<String, ItemGroup> = HashMap::new();

        // Add all items from self
        for item in self.items {
            merged_items.insert(item.id.clone(), item);
        }

        // Add all items from other, merging with existing ones
        for item in other.items {
            let item_id = item.id.clone();
            match merged_items.get(&item_id) {
                Some(existing) => {
                    // Merge with existing item using recursion
                    let merged = existing.clone() + item;
                    merged_items.insert(item_id, merged);
                }
                None => {
                    // Add new item
                    merged_items.insert(item_id, item);
                }
            }
        }

        let mut merged_items_vec: Vec<ItemGroup> = merged_items.into_values().collect();
        merged_items_vec.sort_by(|a, b| a.name.cmp(&b.name));

        // Calculate total size from merged child items
        let total_size_kb = merged_items_vec
            .iter()
            .map(|item| item.size_kb)
            .sum::<u64>();

        Self {
            id: self.id,
            name: if self.name.is_empty() {
                other.name
            } else {
                self.name
            },
            size_kb: total_size_kb,
            items: merged_items_vec,
            leaf: self.leaf && other.leaf, // Only leaf if both are leaf
        }
    }
}

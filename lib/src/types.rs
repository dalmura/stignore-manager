use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::Add;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemGroup {
    pub id: String,
    pub name: String,
    pub size_kb: u64,
    pub items: Vec<ItemGroup>,
    pub leaf: bool,
    #[serde(default)]
    pub copy_count: u8,
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

impl ItemGroup {
    pub fn has_insufficient_copies(&self, minimum_copies: u8) -> bool {
        if self.copy_count < minimum_copies {
            return true;
        }

        self.items
            .iter()
            .any(|item| item.has_insufficient_copies(minimum_copies))
    }
}

impl Add for ItemGroup {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        use std::collections::HashMap;

        let mut merged_items: HashMap<String, ItemGroup> = HashMap::new();

        for item in self.items {
            merged_items.insert(item.id.clone(), item);
        }

        for item in other.items {
            let item_id = item.id.clone();
            match merged_items.get(&item_id) {
                Some(existing) => {
                    let merged = existing.clone() + item;
                    merged_items.insert(item_id, merged);
                }
                None => {
                    merged_items.insert(item_id, item);
                }
            }
        }

        let mut merged_items_vec: Vec<ItemGroup> = merged_items.into_values().collect();
        merged_items_vec.sort_by(|a, b| {
            if a.leaf == b.leaf {
                a.name.cmp(&b.name)
            } else {
                a.leaf.cmp(&b.leaf)
            }
        });

        let total_size_kb = if merged_items_vec.is_empty() {
            if self.size_kb > 0 {
                self.size_kb
            } else {
                other.size_kb
            }
        } else {
            merged_items_vec
                .iter()
                .map(|item| item.size_kb)
                .sum::<u64>()
        };

        let self_empty = self.id.is_empty();
        let other_empty = other.id.is_empty();

        Self {
            id: if self.id.is_empty() {
                other.id
            } else {
                self.id
            },
            name: if self.name.is_empty() {
                other.name
            } else {
                self.name
            },
            size_kb: total_size_kb,
            items: merged_items_vec,
            leaf: self.leaf && other.leaf,
            copy_count: if self_empty {
                other.copy_count
            } else if other_empty {
                self.copy_count
            } else {
                self.copy_count + other.copy_count
            },
        }
    }
}

// Agent API request/response types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryListingResponse {
    pub items: Vec<ItemGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfoResponse {
    pub name: String,
    pub items: Vec<ItemGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemInfoRequest {
    pub item_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemInfoResponse {
    pub item: ItemGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotFoundResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IgnoreRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IgnoreResponse {
    pub success: bool,
    pub message: String,
    pub ignored_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IgnoreStatusRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IgnoreStatusResponse {
    pub ignored: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BulkIgnoreStatusRequest {
    pub items: Vec<IgnoreStatusRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BulkIgnoreStatusItem {
    pub category_id: String,
    pub folder_path: Vec<String>,
    pub ignored: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BulkIgnoreStatusResponse {
    pub items: Vec<BulkIgnoreStatusItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
    pub deleted_path: Option<String>,
}

// Manager-side agent API types (for communicating with agents)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentCategoryListingResponse {
    pub items: Vec<ItemGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentItemInfoRequest {
    pub item_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentItemInfoResponse {
    pub item: ItemGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentIgnoreRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentIgnoreResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentDeleteRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentDeleteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentIgnoreStatusRequest {
    pub category_id: String,
    pub folder_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentIgnoreStatusResponse {
    pub ignored: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentBulkIgnoreStatusRequest {
    pub items: Vec<AgentIgnoreStatusRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentBulkIgnoreStatusResponse {
    pub items: Vec<AgentIgnoreStatusResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_item_group_equality_and_hash() {
        let item1 = ItemGroup {
            id: "same-id".to_string(),
            name: "name1".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let item2 = ItemGroup {
            id: "same-id".to_string(),
            name: "name2".to_string(),
            size_kb: 200,
            items: vec![],
            leaf: false,
            copy_count: 2,
        };

        assert_eq!(item1, item2);
    }

    #[test]
    fn test_has_insufficient_copies_simple() {
        let item = ItemGroup {
            id: "test".to_string(),
            name: "Test".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 2,
        };

        assert!(!item.has_insufficient_copies(1));
        assert!(!item.has_insufficient_copies(2));
        assert!(item.has_insufficient_copies(3));
    }

    #[test]
    fn test_has_insufficient_copies_recursive() {
        let child_with_1_copy = ItemGroup {
            id: "child".to_string(),
            name: "Child".to_string(),
            size_kb: 50,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let parent_with_3_copies = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 100,
            items: vec![child_with_1_copy],
            leaf: false,
            copy_count: 3,
        };

        assert!(parent_with_3_copies.has_insufficient_copies(2));
        assert!(!parent_with_3_copies.has_insufficient_copies(1));
        assert!(parent_with_3_copies.has_insufficient_copies(4));
    }

    #[test]
    fn test_item_group_add_with_children() {
        let child1 = ItemGroup {
            id: "child1".to_string(),
            name: "Child1".to_string(),
            size_kb: 50,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let child2 = ItemGroup {
            id: "child2".to_string(),
            name: "Child2".to_string(),
            size_kb: 75,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let parent1 = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 0,
            items: vec![child1],
            leaf: false,
            copy_count: 1,
        };

        let parent2 = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 0,
            items: vec![child2],
            leaf: false,
            copy_count: 1,
        };

        let result = parent1 + parent2;
        assert_eq!(result.copy_count, 2);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.size_kb, 125);
    }

    #[test]
    fn test_agent_api_types_serialization() {
        let category_response = AgentCategoryListingResponse {
            items: vec![ItemGroup {
                id: "test".to_string(),
                name: "Test".to_string(),
                size_kb: 100,
                items: vec![],
                leaf: true,
                copy_count: 1,
            }],
        };
        let json = serde_json::to_string(&category_response).unwrap();
        let _: AgentCategoryListingResponse = serde_json::from_str(&json).unwrap();

        let item_request = AgentItemInfoRequest {
            item_path: vec!["path".to_string(), "to".to_string(), "item".to_string()],
        };
        let json = serde_json::to_string(&item_request).unwrap();
        let deserialized: AgentItemInfoRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.item_path.len(), 3);
    }
}

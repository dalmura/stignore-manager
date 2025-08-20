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
        // Check if this item itself has insufficient copies
        if self.copy_count < minimum_copies {
            return true;
        }

        // Recursively check if any child has insufficient copies
        self.items
            .iter()
            .any(|item| item.has_insufficient_copies(minimum_copies))
    }
}

impl Add for ItemGroup {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        use std::collections::HashMap;

        // Merge items by id, using addition for duplicates
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
        crate::agents::sort_all_items(&mut merged_items_vec);

        // Calculate total size from merged child items
        let total_size_kb = if merged_items_vec.is_empty() {
            // If no children, use the size from self or other (they should be the same for leaf nodes)
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
            leaf: self.leaf && other.leaf, // Only leaf if both are leaf
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

// Agent API types
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
            name: "name2".to_string(), // Different name but same id
            size_kb: 200,
            items: vec![],
            leaf: false,
            copy_count: 2,
        };

        assert_eq!(item1, item2); // Equality based on id only
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

        // When minimum is 2: parent has 3 (sufficient), but child has 1 (insufficient)
        // Should return true because ANY child has insufficient copies
        assert!(parent_with_3_copies.has_insufficient_copies(2));

        // When minimum is 1: parent has 3 (sufficient), child has 1 (sufficient)
        // Should return false because all items have sufficient copies
        assert!(!parent_with_3_copies.has_insufficient_copies(1));

        // When minimum is 4: parent has 3 (insufficient), child has 1 (also insufficient)
        // Should return true because parent itself has insufficient copies
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
            size_kb: 0, // Will be calculated from children
            items: vec![child1],
            leaf: false,
            copy_count: 1,
        };

        let parent2 = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 0, // Will be calculated from children
            items: vec![child2],
            leaf: false,
            copy_count: 1,
        };

        let result = parent1 + parent2;
        assert_eq!(result.copy_count, 2);
        assert_eq!(result.items.len(), 2); // Two different children
        assert_eq!(result.size_kb, 125); // Sum of child sizes
    }

    #[test]
    fn test_item_group_add_merge_same_children() {
        let movie_a_file = ItemGroup {
            id: "Movie A.mkv".to_string(),
            name: "Movie A.mkv".to_string(),
            size_kb: 50,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let movie_a_folder = ItemGroup {
            id: "Movie A".to_string(),
            name: "Movie A".to_string(),
            size_kb: 50,
            items: vec![movie_a_file],
            leaf: false,
            copy_count: 1,
        };

        let agent_1_response = ItemGroup {
            id: "movies".to_string(),
            name: "Movies".to_string(),
            size_kb: 50,
            items: vec![movie_a_folder.clone()],
            leaf: false,
            copy_count: 1,
        };

        let agent_2_response = ItemGroup {
            id: "movies".to_string(),
            name: "Movies".to_string(),
            size_kb: 50,
            items: vec![movie_a_folder],
            leaf: false,
            copy_count: 1,
        };

        let result = agent_1_response + agent_2_response;
        assert_eq!(result.copy_count, 2);

        // The Movie's deduplicated
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].copy_count, 2);

        // The Movie's file's deduplicated
        assert_eq!(result.items[0].items.len(), 1);
        assert_eq!(result.items[0].items[0].copy_count, 2);

        // The overall size should be the deduplicated size of the leafs
        assert_eq!(result.size_kb, 50);
    }

    #[test]
    fn test_agent_api_types_serialization() {
        // Test AgentCategoryListingResponse
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

        // Test AgentItemInfoRequest
        let item_request = AgentItemInfoRequest {
            item_path: vec!["path".to_string(), "to".to_string(), "item".to_string()],
        };
        let json = serde_json::to_string(&item_request).unwrap();
        let deserialized: AgentItemInfoRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.item_path.len(), 3);

        // Test AgentIgnoreRequest
        let ignore_request = AgentIgnoreRequest {
            category_id: "cat1".to_string(),
            folder_path: vec!["folder".to_string()],
        };
        let json = serde_json::to_string(&ignore_request).unwrap();
        let _: AgentIgnoreRequest = serde_json::from_str(&json).unwrap();

        // Test AgentIgnoreResponse
        let ignore_response = AgentIgnoreResponse {
            success: true,
            message: "Success".to_string(),
        };
        let json = serde_json::to_string(&ignore_response).unwrap();
        let deserialized: AgentIgnoreResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.message, "Success");
    }

    #[test]
    fn test_bulk_ignore_status_types() {
        let status_request = AgentIgnoreStatusRequest {
            category_id: "cat1".to_string(),
            folder_path: vec!["test".to_string()],
        };

        let bulk_request = AgentBulkIgnoreStatusRequest {
            items: vec![status_request],
        };

        let json = serde_json::to_string(&bulk_request).unwrap();
        let deserialized: AgentBulkIgnoreStatusRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.items.len(), 1);

        let status_response = AgentIgnoreStatusResponse { ignored: true };

        let bulk_response = AgentBulkIgnoreStatusResponse {
            items: vec![status_response],
        };

        let json = serde_json::to_string(&bulk_response).unwrap();
        let deserialized: AgentBulkIgnoreStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.items.len(), 1);
        assert!(deserialized.items[0].ignored);
    }
}

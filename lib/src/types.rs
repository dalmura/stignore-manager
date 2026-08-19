use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::Add;

pub const AGENT_API_V1_PREFIX: &str = "api/v1";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ItemGroup {
    pub id: String,
    pub name: String,
    pub size_kb: u64,
    pub items: Vec<ItemGroup>,
    pub leaf: bool,
    #[serde(default)]
    pub copy_count: u8,
    #[serde(default)]
    pub has_conflicts: bool,
    #[serde(default)]
    pub conflict_count: u32,
    #[serde(default)]
    pub is_syncing: bool,
    #[serde(default)]
    pub stversions_size_kb: u64,
    #[serde(default)]
    pub stfolder_present: bool,
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
    pub fn empty() -> Self {
        Self::default()
    }

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
            has_conflicts: self.has_conflicts || other.has_conflicts,
            conflict_count: self.conflict_count + other.conflict_count,
            is_syncing: self.is_syncing || other.is_syncing,
            stversions_size_kb: self.stversions_size_kb + other.stversions_size_kb,
            stfolder_present: self.stfolder_present || other.stfolder_present,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    NameAsc,
    NameDesc,
    SizeDesc,
    SizeAsc,
}

impl SortOrder {
    pub fn from_query(s: Option<&str>) -> Self {
        match s.map(|v| v.trim().to_lowercase()).as_deref() {
            Some("name_desc") | Some("name-desc") => SortOrder::NameDesc,
            Some("size_desc") | Some("size-desc") | Some("size") | Some("biggest") => {
                SortOrder::SizeDesc
            }
            Some("size_asc") | Some("size-asc") | Some("smallest") => SortOrder::SizeAsc,
            Some("name_asc") | Some("name-asc") | Some("name") => SortOrder::NameAsc,
            _ => SortOrder::NameAsc,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::NameAsc => "name_asc",
            SortOrder::NameDesc => "name_desc",
            SortOrder::SizeDesc => "size_desc",
            SortOrder::SizeAsc => "size_asc",
        }
    }

    pub fn sort_items(&self, items: &mut [ItemGroup]) {
        match self {
            SortOrder::NameAsc => {
                items.sort_by(|a, b| {
                    if a.leaf == b.leaf {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        a.leaf.cmp(&b.leaf)
                    }
                });
            }
            SortOrder::NameDesc => {
                items.sort_by(|a, b| {
                    if a.leaf == b.leaf {
                        b.name.to_lowercase().cmp(&a.name.to_lowercase())
                    } else {
                        a.leaf.cmp(&b.leaf)
                    }
                });
            }
            SortOrder::SizeDesc => {
                items.sort_by(|a, b| {
                    b.size_kb
                        .cmp(&a.size_kb)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            SortOrder::SizeAsc => {
                items.sort_by(|a, b| {
                    a.size_kb
                        .cmp(&b.size_kb)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
        }
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

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

pub type AgentUnignoreRequest = AgentIgnoreRequest;
pub type AgentUnignoreResponse = AgentIgnoreResponse;

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
            ..Default::default()
        };

        let item2 = ItemGroup {
            id: "same-id".to_string(),
            name: "name2".to_string(),
            size_kb: 200,
            items: vec![],
            leaf: false,
            copy_count: 2,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };

        let parent_with_3_copies = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 100,
            items: vec![child_with_1_copy],
            leaf: false,
            copy_count: 3,
            ..Default::default()
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
            ..Default::default()
        };

        let child2 = ItemGroup {
            id: "child2".to_string(),
            name: "Child2".to_string(),
            size_kb: 75,
            items: vec![],
            leaf: true,
            copy_count: 1,
            ..Default::default()
        };

        let parent1 = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 0,
            items: vec![child1],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };

        let parent2 = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 0,
            items: vec![child2],
            leaf: false,
            copy_count: 1,
            ..Default::default()
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
                ..Default::default()
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

    #[test]
    fn test_sort_order_from_query() {
        assert_eq!(SortOrder::from_query(None), SortOrder::NameAsc);
        assert_eq!(SortOrder::from_query(Some("")), SortOrder::NameAsc);
        assert_eq!(SortOrder::from_query(Some("name_asc")), SortOrder::NameAsc);
        assert_eq!(SortOrder::from_query(Some("name")), SortOrder::NameAsc);
        assert_eq!(
            SortOrder::from_query(Some("name_desc")),
            SortOrder::NameDesc
        );
        assert_eq!(
            SortOrder::from_query(Some("size_desc")),
            SortOrder::SizeDesc
        );
        assert_eq!(SortOrder::from_query(Some("size")), SortOrder::SizeDesc);
        assert_eq!(SortOrder::from_query(Some("biggest")), SortOrder::SizeDesc);
        assert_eq!(SortOrder::from_query(Some("size_asc")), SortOrder::SizeAsc);
        assert_eq!(SortOrder::from_query(Some("smallest")), SortOrder::SizeAsc);
    }

    #[test]
    fn test_sort_order_sorting() {
        let item_small = ItemGroup {
            id: "small".to_string(),
            name: "B Small".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };
        let item_big = ItemGroup {
            id: "big".to_string(),
            name: "A Big".to_string(),
            size_kb: 5000,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };
        let item_medium = ItemGroup {
            id: "medium".to_string(),
            name: "C Medium".to_string(),
            size_kb: 1000,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };

        let original = vec![item_small.clone(), item_big.clone(), item_medium.clone()];

        // Size Desc (Biggest first)
        let mut by_size_desc = original.clone();
        SortOrder::SizeDesc.sort_items(&mut by_size_desc);
        assert_eq!(by_size_desc[0].name, "A Big");
        assert_eq!(by_size_desc[1].name, "C Medium");
        assert_eq!(by_size_desc[2].name, "B Small");

        // Size Asc (Smallest first)
        let mut by_size_asc = original.clone();
        SortOrder::SizeAsc.sort_items(&mut by_size_asc);
        assert_eq!(by_size_asc[0].name, "B Small");
        assert_eq!(by_size_asc[1].name, "C Medium");
        assert_eq!(by_size_asc[2].name, "A Big");

        // Name Asc (A -> Z)
        let mut by_name_asc = original.clone();
        SortOrder::NameAsc.sort_items(&mut by_name_asc);
        assert_eq!(by_name_asc[0].name, "A Big");
        assert_eq!(by_name_asc[1].name, "B Small");
        assert_eq!(by_name_asc[2].name, "C Medium");

        // Name Desc (Z -> A)
        let mut by_name_desc = original.clone();
        SortOrder::NameDesc.sort_items(&mut by_name_desc);
        assert_eq!(by_name_desc[0].name, "C Medium");
        assert_eq!(by_name_desc[1].name, "B Small");
        assert_eq!(by_name_desc[2].name, "A Big");
    }

    #[test]
    fn test_sort_order_tie_breaking() {
        let item1 = ItemGroup {
            id: "z".to_string(),
            name: "Zeta".to_string(),
            size_kb: 500,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };
        let item2 = ItemGroup {
            id: "a".to_string(),
            name: "Alpha".to_string(),
            size_kb: 500,
            items: vec![],
            leaf: false,
            copy_count: 1,
            ..Default::default()
        };

        let mut items = vec![item1, item2];
        SortOrder::SizeDesc.sort_items(&mut items);
        // Ties broken by name A-Z
        assert_eq!(items[0].name, "Alpha");
        assert_eq!(items[1].name, "Zeta");
    }

    #[test]
    fn test_syncthing_metadata_merging() {
        let item1 = ItemGroup {
            id: "folder".to_string(),
            name: "Folder".to_string(),
            size_kb: 1000,
            items: vec![],
            leaf: false,
            copy_count: 1,
            has_conflicts: true,
            conflict_count: 2,
            is_syncing: false,
            stversions_size_kb: 500,
            stfolder_present: true,
        };

        let item2 = ItemGroup {
            id: "folder".to_string(),
            name: "Folder".to_string(),
            size_kb: 1000,
            items: vec![],
            leaf: false,
            copy_count: 1,
            has_conflicts: false,
            conflict_count: 0,
            is_syncing: true,
            stversions_size_kb: 300,
            stfolder_present: false,
        };

        let merged = item1 + item2;
        assert_eq!(merged.copy_count, 2);
        assert!(merged.has_conflicts);
        assert_eq!(merged.conflict_count, 2);
        assert!(merged.is_syncing);
        assert_eq!(merged.stversions_size_kb, 800);
        assert!(merged.stfolder_present);
    }

    #[test]
    fn test_syncthing_metadata_serialization() {
        let item = ItemGroup {
            id: "sync_item".to_string(),
            name: "Sync Item".to_string(),
            size_kb: 500,
            items: vec![],
            leaf: true,
            copy_count: 1,
            has_conflicts: true,
            conflict_count: 3,
            is_syncing: true,
            stversions_size_kb: 1024,
            stfolder_present: true,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"has_conflicts\":true"));
        assert!(json.contains("\"conflict_count\":3"));
        assert!(json.contains("\"is_syncing\":true"));
        assert!(json.contains("\"stversions_size_kb\":1024"));
        assert!(json.contains("\"stfolder_present\":true"));

        let deserialized: ItemGroup = serde_json::from_str(&json).unwrap();
        assert!(deserialized.has_conflicts);
        assert_eq!(deserialized.conflict_count, 3);
        assert!(deserialized.is_syncing);
        assert_eq!(deserialized.stversions_size_kb, 1024);
        assert!(deserialized.stfolder_present);
    }
}

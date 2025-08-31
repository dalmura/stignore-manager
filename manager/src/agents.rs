use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use stignore_lib::*;

fn set_copy_count_recursive(item: &mut ItemGroup, count: u8) {
    item.copy_count = count;
    for sub_item in &mut item.items {
        set_copy_count_recursive(sub_item, count);
    }
}

pub(crate) fn sort_all_items(items: &mut [ItemGroup]) {
    // Sort items by name
    items.sort_by(|a, b| a.name.cmp(&b.name));

    // Recursively sort all child items
    for item in items {
        sort_all_items(&mut item.items);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryListingResponse {
    pub items: Vec<ItemGroup>,
    pub agent_items: Vec<AgentCategoryListingResponse>,
}

pub async fn list_categories(
    agent_client: &crate::agent_client::AgentClient,
    agents: Vec<Agent>,
) -> CategoryListingResponse {
    let mut agent_responses: Vec<AgentCategoryListingResponse> = vec![];
    let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();

    for agent in agents {
        match agent_client.get_categories(&agent).await {
            Ok(resp) => {
                agent_responses.push(resp.clone());

                for mut item in resp.items {
                    // Use simple copy counting for performance - ignore checking disabled for now
                    set_copy_count_recursive(&mut item, 1);
                    match consolidated.get(&item.id) {
                        Some(existing) => {
                            // Merge with existing item using addition
                            let merged = existing.clone() + item;
                            consolidated.insert(merged.id.clone(), merged);
                        }
                        None => {
                            consolidated.insert(item.id.clone(), item.clone());
                        }
                    };
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get categories from agent '{}': {}",
                    agent.name,
                    e
                );
                // Create an empty response for the failed agent to maintain consistency
                let empty_response = AgentCategoryListingResponse { items: vec![] };
                agent_responses.push(empty_response);
                // Don't add any items to consolidated for this agent
            }
        }
    }

    let mut sorted_items: Vec<ItemGroup> = consolidated.values().cloned().collect();
    sort_all_items(&mut sorted_items);

    CategoryListingResponse {
        agent_items: agent_responses,
        items: sorted_items,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemInfoResponse {
    pub item: ItemGroup,
    pub agent_items: Vec<(Agent, ItemGroup)>,
}

pub async fn item_info(
    agent_client: &crate::agent_client::AgentClient,
    agents: Vec<Agent>,
    item_path: Vec<&str>,
) -> Result<ItemInfoResponse, crate::agent_client::AgentError> {
    let mut agent_responses: Vec<(Agent, ItemGroup)> = vec![];
    let mut consolidated: ItemGroup = ItemGroup {
        id: "".to_string(),
        name: "".to_string(),
        size_kb: 0,
        items: vec![],
        leaf: false,
        copy_count: 0,
    };

    let owned_path: Vec<String> = item_path.iter().map(|v| v.to_string()).collect();

    for agent in agents {
        let body = AgentItemInfoRequest {
            item_path: owned_path.clone(),
        };

        match agent_client.get_item_info(&agent, &body).await {
            Ok(resp) => {
                agent_responses.push((agent, resp.item.clone()));
                let mut item_with_count = resp.item.clone();
                let count = if item_with_count.id.is_empty() { 0 } else { 1 };
                set_copy_count_recursive(&mut item_with_count, count);
                consolidated = item_with_count + consolidated;
            }
            Err(_) => {
                // Handle error by creating an empty response (similar to 404 handling)
                let empty_item = ItemGroup {
                    id: "".to_string(),
                    name: "".to_string(),
                    size_kb: 0,
                    items: vec![],
                    leaf: false,
                    copy_count: 0,
                };
                agent_responses.push((agent, empty_item.clone()));
                consolidated = empty_item + consolidated;
            }
        }
    }

    Ok(ItemInfoResponse {
        agent_items: agent_responses,
        item: consolidated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_copy_count_recursive() {
        let child = ItemGroup {
            id: "child".to_string(),
            name: "Child".to_string(),
            size_kb: 50,
            items: vec![],
            leaf: true,
            copy_count: 0,
        };

        let mut parent = ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 100,
            items: vec![child.clone()],
            leaf: false,
            copy_count: 0,
        };

        set_copy_count_recursive(&mut parent, 3);

        assert_eq!(parent.copy_count, 3);
        assert_eq!(parent.items[0].copy_count, 3);
    }

    #[test]
    fn test_sort_all_items_simple() {
        let mut items = vec![
            ItemGroup {
                id: "z".to_string(),
                name: "Z Item".to_string(),
                size_kb: 100,
                items: vec![],
                leaf: true,
                copy_count: 1,
            },
            ItemGroup {
                id: "a".to_string(),
                name: "A Item".to_string(),
                size_kb: 200,
                items: vec![],
                leaf: true,
                copy_count: 1,
            },
        ];

        sort_all_items(&mut items);

        assert_eq!(items[0].name, "A Item");
        assert_eq!(items[1].name, "Z Item");
    }

    #[test]
    fn test_sort_all_items_recursive() {
        let child_z = ItemGroup {
            id: "child_z".to_string(),
            name: "Z Child".to_string(),
            size_kb: 25,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let child_a = ItemGroup {
            id: "child_a".to_string(),
            name: "A Child".to_string(),
            size_kb: 25,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let mut items = vec![ItemGroup {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            size_kb: 50,
            items: vec![child_z, child_a], // Unsorted children
            leaf: false,
            copy_count: 1,
        }];

        sort_all_items(&mut items);

        // Children should be sorted by name
        assert_eq!(items[0].items[0].name, "A Child");
        assert_eq!(items[0].items[1].name, "Z Child");
    }

    #[test]
    fn test_category_listing_response_serialization() {
        let item = ItemGroup {
            id: "test".to_string(),
            name: "Test".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let agent_response = AgentCategoryListingResponse {
            items: vec![item.clone()],
        };

        let response = CategoryListingResponse {
            items: vec![item],
            agent_items: vec![agent_response],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CategoryListingResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.agent_items.len(), 1);
        assert_eq!(deserialized.items[0].name, "Test");
    }

    #[test]
    fn test_item_info_response_serialization() {
        let agent = Agent {
            name: "Test Agent".to_string(),
            hostname: "localhost:3000".to_string(),
            api_key: "test-key".to_string(),
        };

        let item = ItemGroup {
            id: "test".to_string(),
            name: "Test".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 1,
        };

        let response = ItemInfoResponse {
            item: item.clone(),
            agent_items: vec![(agent, item)],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ItemInfoResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.item.name, "Test");
        assert_eq!(deserialized.agent_items.len(), 1);
        assert_eq!(deserialized.agent_items[0].0.name, "Test Agent");
    }

    // Mock agent client for testing business logic without HTTP calls
    #[allow(dead_code)]
    struct MockAgentClient {
        category_responses:
            std::collections::HashMap<String, Result<AgentCategoryListingResponse, String>>,
        item_responses: std::collections::HashMap<String, Result<AgentItemInfoResponse, String>>,
    }

    #[allow(dead_code)]
    impl MockAgentClient {
        fn new() -> Self {
            Self {
                category_responses: std::collections::HashMap::new(),
                item_responses: std::collections::HashMap::new(),
            }
        }

        fn add_category_response(
            &mut self,
            agent_name: &str,
            response: Result<AgentCategoryListingResponse, String>,
        ) {
            self.category_responses
                .insert(agent_name.to_string(), response);
        }

        fn add_item_response(
            &mut self,
            agent_name: &str,
            response: Result<AgentItemInfoResponse, String>,
        ) {
            self.item_responses.insert(agent_name.to_string(), response);
        }

        async fn get_categories(
            &self,
            agent: &Agent,
        ) -> Result<AgentCategoryListingResponse, crate::agent_client::AgentError> {
            match self.category_responses.get(&agent.name) {
                Some(Ok(response)) => Ok(response.clone()),
                Some(Err(msg)) => Err(crate::agent_client::AgentError::OperationFailed(
                    msg.clone(),
                )),
                None => Err(crate::agent_client::AgentError::OperationFailed(
                    "No mock response configured".to_string(),
                )),
            }
        }

        async fn get_item_info(
            &self,
            agent: &Agent,
            _request: &AgentItemInfoRequest,
        ) -> Result<AgentItemInfoResponse, crate::agent_client::AgentError> {
            match self.item_responses.get(&agent.name) {
                Some(Ok(response)) => Ok(response.clone()),
                Some(Err(msg)) => Err(crate::agent_client::AgentError::OperationFailed(
                    msg.clone(),
                )),
                None => Err(crate::agent_client::AgentError::OperationFailed(
                    "No mock response configured".to_string(),
                )),
            }
        }
    }

    // Test consolidation logic without actual HTTP calls
    #[tokio::test]
    async fn test_consolidation_logic_single_agent() {
        let mut mock_client = MockAgentClient::new();

        let test_item = ItemGroup {
            id: "test-item".to_string(),
            name: "Test Item".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 0, // Will be set by consolidation logic
        };

        mock_client.add_category_response(
            "agent1",
            Ok(AgentCategoryListingResponse {
                items: vec![test_item.clone()],
            }),
        );

        let _agent = Agent {
            name: "agent1".to_string(),
            hostname: "localhost:3001".to_string(),
            api_key: "key1".to_string(),
        };

        // Since we can't easily inject the mock client into the real function,
        // let's test the consolidation logic directly
        let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();
        let mut item_with_count = test_item.clone();
        set_copy_count_recursive(&mut item_with_count, 1);
        consolidated.insert(item_with_count.id.clone(), item_with_count);

        let mut sorted_items: Vec<ItemGroup> = consolidated.values().cloned().collect();
        sort_all_items(&mut sorted_items);

        assert_eq!(sorted_items.len(), 1);
        assert_eq!(sorted_items[0].copy_count, 1);
        assert_eq!(sorted_items[0].name, "Test Item");
    }

    #[tokio::test]
    async fn test_consolidation_logic_multiple_agents_same_item() {
        let test_item = ItemGroup {
            id: "shared-item".to_string(),
            name: "Shared Item".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 0,
        };

        // Simulate two agents returning the same item
        let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();

        // First agent's item
        let mut item1 = test_item.clone();
        set_copy_count_recursive(&mut item1, 1);
        consolidated.insert(item1.id.clone(), item1);

        // Second agent's item (same ID, should merge)
        let mut item2 = test_item.clone();
        set_copy_count_recursive(&mut item2, 1);
        match consolidated.get(&item2.id) {
            Some(existing) => {
                let merged = existing.clone() + item2;
                consolidated.insert(merged.id.clone(), merged);
            }
            None => {
                consolidated.insert(item2.id.clone(), item2);
            }
        }

        let sorted_items: Vec<ItemGroup> = consolidated.values().cloned().collect();
        assert_eq!(sorted_items.len(), 1);
        assert_eq!(sorted_items[0].copy_count, 2); // Both agents contributed
    }

    #[tokio::test]
    async fn test_consolidation_logic_multiple_agents_different_items() {
        let item1 = ItemGroup {
            id: "item1".to_string(),
            name: "Item 1".to_string(),
            size_kb: 100,
            items: vec![],
            leaf: true,
            copy_count: 0,
        };

        let item2 = ItemGroup {
            id: "item2".to_string(),
            name: "Item 2".to_string(),
            size_kb: 200,
            items: vec![],
            leaf: true,
            copy_count: 0,
        };

        let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();

        // First agent's item
        let mut item1_with_count = item1.clone();
        set_copy_count_recursive(&mut item1_with_count, 1);
        consolidated.insert(item1_with_count.id.clone(), item1_with_count);

        // Second agent's different item
        let mut item2_with_count = item2.clone();
        set_copy_count_recursive(&mut item2_with_count, 1);
        consolidated.insert(item2_with_count.id.clone(), item2_with_count);

        let mut sorted_items: Vec<ItemGroup> = consolidated.values().cloned().collect();
        sort_all_items(&mut sorted_items);

        assert_eq!(sorted_items.len(), 2);
        // Should be sorted by name: "Item 1", "Item 2"
        assert_eq!(sorted_items[0].name, "Item 1");
        assert_eq!(sorted_items[1].name, "Item 2");
        assert_eq!(sorted_items[0].copy_count, 1);
        assert_eq!(sorted_items[1].copy_count, 1);
    }

    #[test]
    fn test_empty_item_handling() {
        let empty_item = ItemGroup {
            id: "".to_string(),
            name: "".to_string(),
            size_kb: 0,
            items: vec![],
            leaf: false,
            copy_count: 0,
        };

        let consolidated = empty_item.clone() + empty_item;
        assert_eq!(consolidated.copy_count, 0);
        assert_eq!(consolidated.size_kb, 0);
        assert!(consolidated.id.is_empty());
    }
}

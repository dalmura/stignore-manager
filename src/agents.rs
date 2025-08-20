use crate::config::Agent;
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub(crate) struct CategoryListingResponse {
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
pub(crate) struct ItemInfoResponse {
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

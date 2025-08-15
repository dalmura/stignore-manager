use crate::config::Agent;
use crate::types::ItemGroup;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn set_copy_count_recursive(item: &mut ItemGroup, count: u8) {
    item.copy_count = count;
    for sub_item in &mut item.items {
        set_copy_count_recursive(sub_item, count);
    }
}

fn agent_endpoint(agent: &Agent, endpoint: &str, secure: bool) -> String {
    let base = match secure {
        true => "https",
        false => "http",
    };

    format!("{base}://{hostname}/{endpoint}", hostname = agent.hostname)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AgentCategoryListingResponse {
    pub items: Vec<ItemGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CategoryListingResponse {
    pub items: Vec<ItemGroup>,
    pub agent_items: Vec<AgentCategoryListingResponse>,
}

pub async fn list_categories(
    agents: Vec<Agent>,
) -> Result<CategoryListingResponse, reqwest::Error> {
    let mut agent_responses: Vec<AgentCategoryListingResponse> = vec![];
    let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();

    for agent in agents {
        let url = agent_endpoint(&agent, "api/v1/categories", false);
        let resp = reqwest::get(&url)
            .await?
            .json::<AgentCategoryListingResponse>()
            .await?;

        agent_responses.push(resp.clone());

        for mut item in resp.items {
            set_copy_count_recursive(&mut item, 1); // Each agent contributes 1 copy
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

    let mut sorted_items: Vec<ItemGroup> = consolidated.values().cloned().collect();
    sorted_items.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(CategoryListingResponse {
        agent_items: agent_responses,
        items: sorted_items,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AgentItemInfoRequest {
    pub item_path: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AgentItemInfoResponse {
    pub item: ItemGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ItemInfoResponse {
    pub item: ItemGroup,
    pub agent_items: Vec<(Agent, ItemGroup)>,
}

pub async fn item_info(
    agents: Vec<Agent>,
    item_path: Vec<&str>,
) -> Result<ItemInfoResponse, reqwest::Error> {
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
        let client = reqwest::Client::new();
        let url = agent_endpoint(&agent, "api/v1/items", false);
        let body = AgentItemInfoRequest {
            item_path: owned_path.clone(),
        };

        let response = client.post(&url).json(&body).send().await?;

        let status = response.status();

        // Handle 404 responses by creating an empty response
        if status == reqwest::StatusCode::NOT_FOUND {
            let empty_item = ItemGroup {
                id: "".to_string(),
                name: "".to_string(),
                size_kb: 0,
                items: vec![],
                leaf: false,
                copy_count: 0,
            };
            let resp = AgentItemInfoResponse { item: empty_item };
            agent_responses.push((agent, resp.item.clone()));
            consolidated = resp.item.clone() + consolidated;
            continue;
        }

        let resp_result = response.json::<AgentItemInfoResponse>().await;

        let resp = match resp_result {
            Ok(parsed) => parsed,
            Err(e) => {
                return Err(e);
            }
        };

        agent_responses.push((agent, resp.item.clone()));
        let mut item_with_count = resp.item.clone();
        let count = if item_with_count.id.is_empty() { 0 } else { 1 };
        set_copy_count_recursive(&mut item_with_count, count);
        consolidated = item_with_count + consolidated;
    }

    Ok(ItemInfoResponse {
        agent_items: agent_responses,
        item: consolidated,
    })
}

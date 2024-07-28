use crate::config::Agent;
use crate::types::ItemGroup;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        for item in resp.items {
            match consolidated.get(&item.id) {
                Some(_) => {}
                None => {
                    consolidated.insert(item.id.clone(), item.clone());
                }
            };
        }
    }

    Ok(CategoryListingResponse {
        agent_items: agent_responses,
        items: consolidated.values().cloned().collect(),
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
    };

    let owned_path: Vec<String> = item_path.iter().map(|v| v.to_string()).collect();

    for agent in agents {
        let client = reqwest::Client::new();
        let url = agent_endpoint(&agent, "api/v1/items", false);
        let body = AgentItemInfoRequest {
            item_path: owned_path.clone(),
        };

        tracing::info!("POST {:?}", &url);
        tracing::info!("{:?}", &body);

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json::<AgentItemInfoResponse>()
            .await?;

        agent_responses.push((agent, resp.item.clone()));
        consolidated = resp.item.clone() + consolidated;
    }

    Ok(ItemInfoResponse {
        agent_items: agent_responses,
        item: consolidated,
    })
}

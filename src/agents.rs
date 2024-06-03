use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::types::ItemGroup;
use crate::config::Agent;

fn agent_endpoint(agent: &Agent, endpoint: &str, secure: bool) -> String {
    let base = match secure {
        true => "https",
        false => "http",
    };

    format!("{base}://{hostname}/{endpoint}", hostname=agent.hostname)
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


pub async fn list_categories(agents: Vec<Agent>) -> Result<CategoryListingResponse, reqwest::Error> {
    let mut agent_responses: Vec<AgentCategoryListingResponse> = vec!();
    let mut consolidated: HashMap<String, ItemGroup> = HashMap::new();

    for agent in agents {
        let url = agent_endpoint(&agent, "api/v1/categories", false);
        let resp = reqwest::get(&url).await?.json::<AgentCategoryListingResponse>().await?;

        agent_responses.push(resp.clone());

        for item in resp.items {
            let _ = match consolidated.get(&item.id) {
                Some(i) => consolidated.insert(item.id.clone(), item.clone() + i.to_owned()),
                None => consolidated.insert(item.id.clone(), item.clone()),
            };
        }
    }

    Ok(
        CategoryListingResponse {
            agent_items: agent_responses,
            items: consolidated.values().cloned().collect(),
        }
    )
}
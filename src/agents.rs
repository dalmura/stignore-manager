use serde::Serialize;
use crate::types::Category;
use crate::config::Agent;

#[derive(Debug, Serialize)]
pub struct AgentListCategoriesResponse {
    pub(crate) categories: Vec<Category>,
}

pub fn list_categories(agents: Vec<Agent>) -> AgentListCategoriesResponse {
    // loop through agents performing request
    let categories = vec![
        Category{name: "Category 1".to_string(), count: 6},
        Category{name: "Category 2".to_string(), count: 12},
        Category{name: "Category 3".to_string(), count: 2},
        Category{name: "Category 4".to_string(), count: 999},
    ];

    AgentListCategoriesResponse {categories}
}
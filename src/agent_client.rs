use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

use crate::config::Agent;
use crate::types::*;

#[derive(Debug)]
pub enum AgentError {
    RequestFailed(reqwest::Error),
    InvalidResponse(String),
    OperationFailed(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::RequestFailed(e) => write!(f, "Request failed: {}", e),
            AgentError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            AgentError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

#[derive(Clone)]
pub struct AgentClient {
    client: Client,
}

impl AgentClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Generic method to make authenticated requests to agents
    async fn make_request<T, R>(
        &self,
        agent: &Agent,
        endpoint: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<R, AgentError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("http://{}/api/v1/{}", agent.hostname, endpoint);

        let mut request = self
            .client
            .request(method, &url)
            .header("X-API-Key", &agent.api_key);

        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .json(body);
        }

        let response = request.send().await.map_err(AgentError::RequestFailed)?;

        if !response.status().is_success() {
            return Err(AgentError::InvalidResponse(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| AgentError::InvalidResponse(e.to_string()))
    }

    /// Get categories from an agent
    pub async fn get_categories(
        &self,
        agent: &Agent,
    ) -> Result<crate::types::AgentCategoryListingResponse, AgentError> {
        self.make_request::<(), _>(agent, "categories", Method::GET, None)
            .await
    }

    /// Get item information from an agent
    pub async fn get_item_info(
        &self,
        agent: &Agent,
        request: &crate::types::AgentItemInfoRequest,
    ) -> Result<crate::types::AgentItemInfoResponse, AgentError> {
        self.make_request(agent, "items", Method::POST, Some(request))
            .await
    }

    /// Ignore an item on an agent
    pub async fn ignore_item(
        &self,
        agent: &Agent,
        request: &AgentIgnoreRequest,
    ) -> Result<AgentIgnoreResponse, AgentError> {
        let response = self
            .make_request(agent, "ignore", Method::POST, Some(request))
            .await?;

        // Check if the operation was successful
        match response {
            AgentIgnoreResponse {
                success: true,
                message,
            } => Ok(AgentIgnoreResponse {
                success: true,
                message,
            }),
            AgentIgnoreResponse {
                success: false,
                message,
            } => Err(AgentError::OperationFailed(message)),
        }
    }

    /// Check ignore status for multiple items in bulk
    pub async fn check_ignore_status_bulk(
        &self,
        agent: &Agent,
        request: &AgentBulkIgnoreStatusRequest,
    ) -> Result<AgentBulkIgnoreStatusResponse, AgentError> {
        self.make_request(agent, "ignore-status-bulk", Method::POST, Some(request))
            .await
    }

    /// Delete an item on an agent
    pub async fn delete_item(
        &self,
        agent: &Agent,
        request: &AgentDeleteRequest,
    ) -> Result<AgentDeleteResponse, AgentError> {
        let response = self
            .make_request(agent, "delete", Method::POST, Some(request))
            .await?;

        // Check if the operation was successful
        match response {
            AgentDeleteResponse {
                success: true,
                message,
            } => Ok(AgentDeleteResponse {
                success: true,
                message,
            }),
            AgentDeleteResponse {
                success: false,
                message,
            } => Err(AgentError::OperationFailed(message)),
        }
    }
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

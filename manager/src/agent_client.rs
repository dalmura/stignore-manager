use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use stignore_lib::*;

#[derive(Debug)]
pub enum AgentError {
    RequestFailed(reqwest::Error),
    Timeout(reqwest::Error),
    InvalidResponse(String),
    NotFound(String),
    OperationFailed(String),
}

impl AgentError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, AgentError::NotFound(_))
    }
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::RequestFailed(e) => write!(f, "Request failed: {}", e),
            AgentError::Timeout(e) => write!(f, "Request timed out: {}", e),
            AgentError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            AgentError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AgentError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

pub const DEFAULT_API_PREFIX: &str = AGENT_API_V1_PREFIX;

#[derive(Clone)]
pub struct AgentClient {
    client: Client,
    api_prefix: String,
}

pub fn format_agent_url(hostname: &str, api_prefix: &str, endpoint: &str) -> String {
    let hostname = hostname.trim();
    let base = if hostname.starts_with("http://") || hostname.starts_with("https://") {
        hostname.trim_end_matches('/').to_string()
    } else if hostname.ends_with(":443") {
        format!("https://{}", hostname.trim_end_matches('/'))
    } else {
        format!("http://{}", hostname.trim_end_matches('/'))
    };

    let prefix = api_prefix.trim_matches('/');
    let endpoint = endpoint.trim_start_matches('/');

    if prefix.is_empty() {
        format!("{}/{}", base, endpoint)
    } else {
        format!("{}/{}/{}", base, prefix, endpoint)
    }
}

impl AgentClient {
    pub fn new() -> Self {
        Self::with_timeout(5)
    }

    pub fn with_timeout(timeout_seconds: u64) -> Self {
        Self::with_timeout_and_prefix(timeout_seconds, DEFAULT_API_PREFIX)
    }

    pub fn with_timeout_and_prefix(timeout_seconds: u64, api_prefix: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()
                .expect("Failed to build HTTP client"),
            api_prefix: api_prefix.into(),
        }
    }

    pub fn api_prefix(&self) -> &str {
        &self.api_prefix
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
        let url = format_agent_url(&agent.hostname, &self.api_prefix, endpoint);
        tracing::debug!(
            "Making {} request to agent '{}' at URL: {}",
            method,
            agent.name,
            url
        );

        let mut request = self
            .client
            .request(method, &url)
            .header("X-API-Key", &agent.api_key);

        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .json(body);
            tracing::debug!("Request to agent '{}' includes JSON body", agent.name);
        }

        tracing::debug!(
            "Sending request to agent '{}' (timeout configured)",
            agent.name
        );
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                tracing::warn!("Request to agent '{}' timed out: {}", agent.name, e);
                AgentError::Timeout(e)
            } else {
                tracing::warn!("Request to agent '{}' failed: {}", agent.name, e);
                AgentError::RequestFailed(e)
            }
        })?;

        let status = response.status();
        tracing::debug!(
            "Received response from agent '{}' with status: {}",
            agent.name,
            status
        );

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::NOT_FOUND {
                tracing::debug!(
                    "Agent '{}' returned status {}: {}",
                    agent.name,
                    status,
                    error_body
                );
                return Err(AgentError::NotFound(error_body));
            } else {
                tracing::error!(
                    "Agent '{}' returned error status {}: {}",
                    agent.name,
                    status,
                    error_body
                );
            }
            return Err(AgentError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        tracing::debug!("Parsing JSON response from agent '{}'", agent.name);
        let result = response.json::<R>().await.map_err(|e| {
            tracing::error!(
                "Failed to parse JSON response from agent '{}': {}",
                agent.name,
                e
            );
            AgentError::InvalidResponse(e.to_string())
        });

        if result.is_ok() {
            tracing::debug!("Successfully parsed response from agent '{}'", agent.name);
        }

        result
    }

    /// Get categories from an agent
    pub async fn get_categories(
        &self,
        agent: &Agent,
    ) -> Result<AgentCategoryListingResponse, AgentError> {
        tracing::info!("Getting categories from agent '{}'", agent.name);
        self.make_request::<(), _>(agent, "categories", Method::GET, None)
            .await
    }

    /// Get item information from an agent
    pub async fn get_item_info(
        &self,
        agent: &Agent,
        request: &AgentItemInfoRequest,
    ) -> Result<AgentItemInfoResponse, AgentError> {
        tracing::info!(
            "Getting item info from agent '{}' for path: {:?}",
            agent.name,
            request.item_path
        );
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

    /// Unignore an item on an agent
    pub async fn unignore_item(
        &self,
        agent: &Agent,
        request: &AgentUnignoreRequest,
    ) -> Result<AgentUnignoreResponse, AgentError> {
        let response = self
            .make_request(agent, "unignore", Method::POST, Some(request))
            .await?;

        match response {
            AgentUnignoreResponse {
                success: true,
                message,
            } => Ok(AgentUnignoreResponse {
                success: true,
                message,
            }),
            AgentUnignoreResponse {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_agent_url_plain_host() {
        assert_eq!(
            format_agent_url("localhost:3000", DEFAULT_API_PREFIX, "categories"),
            "http://localhost:3000/api/v1/categories"
        );
    }

    #[test]
    fn test_format_agent_url_http_prefix() {
        assert_eq!(
            format_agent_url("http://agent.local:3000/", DEFAULT_API_PREFIX, "categories"),
            "http://agent.local:3000/api/v1/categories"
        );
    }

    #[test]
    fn test_format_agent_url_https_prefix() {
        assert_eq!(
            format_agent_url("https://agent.dalmura.cloud", DEFAULT_API_PREFIX, "items"),
            "https://agent.dalmura.cloud/api/v1/items"
        );
    }

    #[test]
    fn test_format_agent_url_port_443_implicit_https() {
        assert_eq!(
            format_agent_url("agent.dalmura.cloud:443", DEFAULT_API_PREFIX, "categories"),
            "https://agent.dalmura.cloud:443/api/v1/categories"
        );
    }

    #[test]
    fn test_format_agent_url_custom_prefix() {
        assert_eq!(
            format_agent_url("agent.dalmura.cloud", "api/v2", "categories"),
            "http://agent.dalmura.cloud/api/v2/categories"
        );
    }

    #[test]
    fn test_format_agent_url_empty_prefix() {
        assert_eq!(
            format_agent_url("https://agent.dalmura.cloud", "", "health"),
            "https://agent.dalmura.cloud/health"
        );
    }
}

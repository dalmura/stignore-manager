use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug)]
pub enum ConfigError {
    FileRead {
        filename: String,
        source: std::io::Error,
    },
    Parse {
        filename: String,
        source: toml::de::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileRead { filename, source } => {
                write!(f, "Could not read config file '{}': {}", filename, source)
            }
            ConfigError::Parse { filename, source } => {
                write!(f, "Unable to parse config file '{}': {}", filename, source)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::FileRead { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub relative_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub port: u16,
    pub base_path: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentData {
    pub agent: AgentConfig,
    pub categories: Vec<Category>,
}

pub fn load_agent_config(filename: &str) -> Result<AgentData, ConfigError> {
    let contents = fs::read_to_string(filename).map_err(|source| ConfigError::FileRead {
        filename: filename.to_string(),
        source,
    })?;

    let data: AgentData = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        filename: filename.to_string(),
        source,
    })?;

    Ok(data)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerData {
    pub manager: ManagerConfig,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerConfig {
    pub port: u16,
    pub minimum_copies: u8,
    #[serde(default = "default_agent_timeout_seconds")]
    pub agent_timeout_seconds: u64,
}

fn default_agent_timeout_seconds() -> u64 {
    5
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub name: String,
    pub hostname: String,
    pub api_key: String,
}

pub fn load_manager_config(filename: &str) -> Result<ManagerData, ConfigError> {
    let contents = fs::read_to_string(filename).map_err(|source| ConfigError::FileRead {
        filename: filename.to_string(),
        source,
    })?;

    let data: ManagerData = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        filename: filename.to_string(),
        source,
    })?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_agent_config_serde() {
        let data: Result<AgentData, toml::de::Error> = toml::from_str(
            r#"
           [agent]
           port = 3000
           name = "Agent Smith"
           base_path = "/path/to/stuff"
           api_key = "550e8400-e29b-41d4-a716-446655440000"
        
           [[categories]]
           id = "category_a"
           name = "Category A"
           relative_path = "a/"
        "#,
        );
        assert!(data.is_ok());
    }

    #[test]
    fn test_manager_config_serde() {
        let data: Result<ManagerData, toml::de::Error> = toml::from_str(
            r#"
                [manager]
                port = 8000
                minimum_copies = 2

                [[agents]]
                name = "Agent 1"
                hostname = "localhost:3000"
                api_key = "550e8400-e29b-41d4-a716-446655440000"
            "#,
        );
        assert!(data.is_ok());
        let config = data.unwrap();
        assert_eq!(config.manager.agent_timeout_seconds, 5);
    }

    #[test]
    fn test_load_agent_config_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
[agent]
port = 3001
name = "Test Agent"
base_path = "/tmp/test"
api_key = "550e8400-e29b-41d4-a716-446655440000"

[[categories]]
id = "test_category"
name = "Test Category"
relative_path = "test/"
        "#;

        temp_file.write_all(config_content.as_bytes()).unwrap();
        let file_path = temp_file.path().to_str().unwrap();

        let result = load_agent_config(file_path);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.agent.port, 3001);
        assert_eq!(data.agent.name, "Test Agent");
        assert_eq!(data.categories.len(), 1);
        assert_eq!(data.categories[0].id, "test_category");
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_agent_config("nonexistent_file.toml");
        assert!(result.is_err());

        match result.unwrap_err() {
            ConfigError::FileRead { filename, .. } => {
                assert_eq!(filename, "nonexistent_file.toml");
            }
            _ => panic!("Expected FileRead error"),
        }
    }
}

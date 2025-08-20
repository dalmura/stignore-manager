use serde::{Deserialize, Serialize};
use std::fs;
use std::process::exit;

// Parent struct holding the entire config file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
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

pub fn load_config(filename: &str) -> Data {
    let contents = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Could not read file `{}`", filename);
            exit(1);
        }
    };

    let data: Data = match toml::from_str(&contents) {
        Ok(d) => d,
        Err(a) => {
            eprintln!("Unable to load data from `{}`", filename);
            eprintln!("`{}`", a);
            exit(1);
        }
    };

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_valid_config() {
        let data: Result<Data, toml::de::Error> = toml::from_str(
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
        // Test default timeout is applied when not specified
        assert_eq!(config.manager.agent_timeout_seconds, 5);
    }

    #[test]
    fn serde_config_with_custom_timeout() {
        let data: Result<Data, toml::de::Error> = toml::from_str(
            r#"
                [manager]
                port = 8000
                minimum_copies = 2
                agent_timeout_seconds = 10

                [[agents]]
                name = "Agent 1"
                hostname = "localhost:3000"
                api_key = "550e8400-e29b-41d4-a716-446655440000"
            "#,
        );
        assert!(data.is_ok());
        let config = data.unwrap();
        // Test custom timeout is applied
        assert_eq!(config.manager.agent_timeout_seconds, 10);
    }

    #[test]
    fn serde_invalid_config() {
        let data: Result<Data, toml::de::Error> = toml::from_str(
            r#"
               [manager]
               port = 3000
               fake_field = 0
            "#,
        );
        assert!(data.is_err());
    }
}

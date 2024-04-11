use serde::{Deserialize, Serialize};
use std::fs;
use std::process::exit;

// Parent struct holding the entire config file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    pub(crate) manager: ManagerConfig,
    pub(crate) agents: Vec<Agent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerConfig {
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    hostname: String,
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
               port = 3000
            "#,
        );
        assert!(data.is_ok());
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

use std::process::exit;
use stignore_lib::{ManagerData, load_manager_config, resolve_config_path};

pub fn load_config(explicit_filename: Option<&str>) -> ManagerData {
    let default_paths = [
        "/app/config.toml",
        "/config.toml",
        "config.toml",
        "manager/config.toml",
    ];

    let path = match resolve_config_path(explicit_filename, &default_paths) {
        Some(p) => p,
        None => {
            eprintln!(
                "Failed to find configuration file. Please provide a path as an argument, set STIGNORE_CONFIG, or place config.toml in a standard location (/app/config.toml, /config.toml, ./config.toml)."
            );
            exit(1);
        }
    };

    match load_manager_config(&path) {
        Ok(data) => {
            tracing::info!("Loaded configuration from {}", path);
            data
        }
        Err(err) => {
            eprintln!("Failed to load configuration from '{}': {}", path, err);
            exit(1);
        }
    }
}

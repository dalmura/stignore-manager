use std::process::exit;
use stignore_lib::{load_manager_config, ManagerData};

pub fn load_config(filename: &str) -> ManagerData {
    match load_manager_config(filename) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load configuration: {}", err);
            exit(1);
        }
    }
}

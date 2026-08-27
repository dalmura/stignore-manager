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

/// Expands environment variable placeholders in text.
/// Supports both `${VAR_NAME}` and `$VAR_NAME` syntax.
/// If an environment variable is not set, the placeholder remains unchanged.
pub fn expand_env_vars(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut var_name = String::new();
                let mut closed = false;
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next(); // consume '}'
                        closed = true;
                        break;
                    }
                    if c.is_alphanumeric() || c == '_' {
                        var_name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if closed && !var_name.is_empty() {
                    match std::env::var(&var_name) {
                        Ok(val) => result.push_str(&val),
                        Err(_) => {
                            result.push('$');
                            result.push('{');
                            result.push_str(&var_name);
                            result.push('}');
                        }
                    }
                } else {
                    result.push('$');
                    result.push('{');
                    result.push_str(&var_name);
                }
            } else {
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        var_name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !var_name.is_empty() {
                    match std::env::var(&var_name) {
                        Ok(val) => result.push_str(&val),
                        Err(_) => {
                            result.push('$');
                            result.push_str(&var_name);
                        }
                    }
                } else {
                    result.push('$');
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Resolves the configuration file path from explicit arguments, environment variables,
/// or a list of default search paths.
pub fn resolve_config_path(explicit: Option<&str>, default_locations: &[&str]) -> Option<String> {
    if let Some(path) = explicit.filter(|p| !p.is_empty()) {
        return Some(path.to_string());
    }

    if let Some(path) = std::env::var("STIGNORE_CONFIG")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    {
        return Some(path);
    }

    if let Some(path) = std::env::var("CONFIG_PATH")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    {
        return Some(path);
    }

    for path in default_locations {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

/// Applies environment variable overrides to a ManagerData configuration.
pub fn apply_manager_env_overrides(mut data: ManagerData) -> ManagerData {
    if let Ok(port) = std::env::var("STIGNORE_PORT")
        .or_else(|_| std::env::var("PORT"))
        .or_else(|_| std::env::var("STIGNORE_MANAGER_PORT"))
        .and_then(|s| s.parse::<u16>().map_err(|_| std::env::VarError::NotPresent))
    {
        data.manager.port = port;
    }

    if let Ok(min_copies) = std::env::var("STIGNORE_MINIMUM_COPIES")
        .and_then(|s| s.parse::<u8>().map_err(|_| std::env::VarError::NotPresent))
    {
        data.manager.minimum_copies = min_copies;
    }

    if let Ok(timeout) = std::env::var("STIGNORE_AGENT_TIMEOUT_SECONDS")
        .and_then(|s| s.parse::<u64>().map_err(|_| std::env::VarError::NotPresent))
    {
        data.manager.agent_timeout_seconds = timeout;
    }

    if let Ok(auth_enabled_str) = std::env::var("STIGNORE_AUTH_ENABLED") {
        let is_enabled = match auth_enabled_str.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        };
        if let Some(enabled) = is_enabled {
            data.manager.auth.enabled = enabled;
        }
    }

    if let Ok(user_header) = std::env::var("STIGNORE_AUTH_USER_HEADER") {
        data.manager.auth.user_header = user_header;
    }

    if let Ok(role_header) = std::env::var("STIGNORE_AUTH_ROLE_HEADER") {
        data.manager.auth.role_header = role_header;
    }

    if let Ok(admin_role) = std::env::var("STIGNORE_AUTH_ADMIN_ROLE") {
        data.manager.auth.admin_role = admin_role;
    }

    if let Ok(reader_role) = std::env::var("STIGNORE_AUTH_READER_ROLE") {
        data.manager.auth.reader_role = reader_role;
    }

    // Radarr environment overrides
    if let Ok(url) = std::env::var("STIGNORE_RADARR_URL").or_else(|_| std::env::var("RADARR_URL")) {
        let mut radarr = data.integrations.radarr.unwrap_or_default();
        radarr.url = url;
        radarr.enabled = true;
        data.integrations.radarr = Some(radarr);
    }

    if let Ok(api_key) =
        std::env::var("STIGNORE_RADARR_API_KEY").or_else(|_| std::env::var("RADARR_API_KEY"))
    {
        let mut radarr = data.integrations.radarr.unwrap_or_default();
        radarr.api_key = api_key;
        data.integrations.radarr = Some(radarr);
    }

    if let Ok(cat) =
        std::env::var("STIGNORE_RADARR_CATEGORY").or_else(|_| std::env::var("RADARR_CATEGORY"))
    {
        let mut radarr = data.integrations.radarr.unwrap_or_default();
        radarr.category_id = cat;
        data.integrations.radarr = Some(radarr);
    }

    if let Ok(enabled_str) =
        std::env::var("STIGNORE_RADARR_ENABLED").or_else(|_| std::env::var("RADARR_ENABLED"))
        && let Some(enabled) = match enabled_str.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    {
        let mut radarr = data.integrations.radarr.unwrap_or_default();
        radarr.enabled = enabled;
        data.integrations.radarr = Some(radarr);
    }

    if let Ok(excl_str) = std::env::var("STIGNORE_RADARR_ADD_IMPORT_EXCLUSION")
        .or_else(|_| std::env::var("RADARR_ADD_IMPORT_EXCLUSION"))
        && let Some(excl) = match excl_str.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    {
        let mut radarr = data.integrations.radarr.unwrap_or_default();
        radarr.add_import_exclusion = excl;
        data.integrations.radarr = Some(radarr);
    }

    // Sonarr environment overrides
    if let Ok(url) = std::env::var("STIGNORE_SONARR_URL").or_else(|_| std::env::var("SONARR_URL")) {
        let mut sonarr = data.integrations.sonarr.unwrap_or_default();
        sonarr.url = url;
        sonarr.enabled = true;
        data.integrations.sonarr = Some(sonarr);
    }

    if let Ok(api_key) =
        std::env::var("STIGNORE_SONARR_API_KEY").or_else(|_| std::env::var("SONARR_API_KEY"))
    {
        let mut sonarr = data.integrations.sonarr.unwrap_or_default();
        sonarr.api_key = api_key;
        data.integrations.sonarr = Some(sonarr);
    }

    if let Ok(cat) =
        std::env::var("STIGNORE_SONARR_CATEGORY").or_else(|_| std::env::var("SONARR_CATEGORY"))
    {
        let mut sonarr = data.integrations.sonarr.unwrap_or_default();
        sonarr.category_id = cat;
        data.integrations.sonarr = Some(sonarr);
    }

    if let Ok(enabled_str) =
        std::env::var("STIGNORE_SONARR_ENABLED").or_else(|_| std::env::var("SONARR_ENABLED"))
        && let Some(enabled) = match enabled_str.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    {
        let mut sonarr = data.integrations.sonarr.unwrap_or_default();
        sonarr.enabled = enabled;
        data.integrations.sonarr = Some(sonarr);
    }

    if let Ok(excl_str) = std::env::var("STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION")
        .or_else(|_| std::env::var("SONARR_ADD_IMPORT_LIST_EXCLUSION"))
        && let Some(excl) = match excl_str.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    {
        let mut sonarr = data.integrations.sonarr.unwrap_or_default();
        sonarr.add_import_list_exclusion = excl;
        data.integrations.sonarr = Some(sonarr);
    }

    data
}

pub fn load_agent_config(filename: &str) -> Result<AgentData, ConfigError> {
    let contents = fs::read_to_string(filename).map_err(|source| ConfigError::FileRead {
        filename: filename.to_string(),
        source,
    })?;

    let expanded = expand_env_vars(&contents);

    let data: AgentData = toml::from_str(&expanded).map_err(|source| ConfigError::Parse {
        filename: filename.to_string(),
        source,
    })?;

    Ok(data)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerData {
    pub manager: ManagerConfig,
    #[serde(default)]
    pub integrations: IntegrationsConfig,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct IntegrationsConfig {
    #[serde(default)]
    pub radarr: Option<RadarrConfig>,
    #[serde(default)]
    pub sonarr: Option<SonarrConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RadarrConfig {
    #[serde(default = "default_integration_enabled")]
    pub enabled: bool,
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_radarr_category")]
    pub category_id: String,
    #[serde(default)]
    pub delete_files: bool,
    #[serde(default)]
    pub add_import_exclusion: bool,
}

impl Default for RadarrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: String::new(),
            api_key: String::new(),
            category_id: default_radarr_category(),
            delete_files: false,
            add_import_exclusion: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SonarrConfig {
    #[serde(default = "default_integration_enabled")]
    pub enabled: bool,
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_sonarr_category")]
    pub category_id: String,
    #[serde(default)]
    pub delete_files: bool,
    #[serde(default)]
    pub add_import_list_exclusion: bool,
}

impl Default for SonarrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: String::new(),
            api_key: String::new(),
            category_id: default_sonarr_category(),
            delete_files: false,
            add_import_list_exclusion: false,
        }
    }
}

fn default_integration_enabled() -> bool {
    true
}

fn default_radarr_category() -> String {
    "movies".to_string()
}

fn default_sonarr_category() -> String {
    "tv".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerConfig {
    pub port: u16,
    pub minimum_copies: u8,
    #[serde(default = "default_agent_timeout_seconds")]
    pub agent_timeout_seconds: u64,
    #[serde(default)]
    pub auth: AuthConfig,
}

fn default_agent_timeout_seconds() -> u64 {
    5
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_user_header")]
    pub user_header: String,
    #[serde(default = "default_role_header")]
    pub role_header: String,
    #[serde(default = "default_admin_role")]
    pub admin_role: String,
    #[serde(default = "default_reader_role")]
    pub reader_role: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            user_header: default_user_header(),
            role_header: default_role_header(),
            admin_role: default_admin_role(),
            reader_role: default_reader_role(),
        }
    }
}

fn default_user_header() -> String {
    "X-Proxy-User".to_string()
}

fn default_role_header() -> String {
    "X-Proxy-Role".to_string()
}

fn default_admin_role() -> String {
    "Admin".to_string()
}

fn default_reader_role() -> String {
    "Reader".to_string()
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

    let expanded = expand_env_vars(&contents);

    let data: ManagerData = toml::from_str(&expanded).map_err(|source| ConfigError::Parse {
        filename: filename.to_string(),
        source,
    })?;

    Ok(apply_manager_env_overrides(data))
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
        assert!(!config.manager.auth.enabled);
        assert_eq!(config.manager.auth.user_header, "X-Proxy-User");
        assert_eq!(config.manager.auth.role_header, "X-Proxy-Role");
    }

    #[test]
    fn test_manager_config_with_auth_serde() {
        let data: Result<ManagerData, toml::de::Error> = toml::from_str(
            r#"
                [manager]
                port = 8000
                minimum_copies = 2

                [manager.auth]
                enabled = true
                user_header = "X-Authentik-Username"
                role_header = "X-Authentik-Groups"
                admin_role = "stignore-admins"
                reader_role = "stignore-readers"

                [[agents]]
                name = "Agent 1"
                hostname = "localhost:3000"
                api_key = "550e8400-e29b-41d4-a716-446655440000"
            "#,
        );
        assert!(data.is_ok());
        let config = data.unwrap();
        assert!(config.manager.auth.enabled);
        assert_eq!(config.manager.auth.user_header, "X-Authentik-Username");
        assert_eq!(config.manager.auth.role_header, "X-Authentik-Groups");
        assert_eq!(config.manager.auth.admin_role, "stignore-admins");
        assert_eq!(config.manager.auth.reader_role, "stignore-readers");
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

    #[test]
    fn test_expand_env_vars() {
        unsafe {
            std::env::set_var("TEST_STIGNORE_PORT", "9999");
            std::env::set_var("TEST_STIGNORE_KEY", "secret-test-key");
        }

        let input = "port = ${TEST_STIGNORE_PORT}\nkey = \"$TEST_STIGNORE_KEY\"\nother = \"${UNSET_VAR_XYZ}\"";
        let expanded = expand_env_vars(input);

        assert_eq!(
            expanded,
            "port = 9999\nkey = \"secret-test-key\"\nother = \"${UNSET_VAR_XYZ}\""
        );

        unsafe {
            std::env::remove_var("TEST_STIGNORE_PORT");
            std::env::remove_var("TEST_STIGNORE_KEY");
        }
    }

    #[test]
    fn test_apply_manager_env_overrides() {
        let base = ManagerData {
            manager: ManagerConfig {
                port: 8000,
                minimum_copies: 2,
                agent_timeout_seconds: 5,
                auth: AuthConfig::default(),
            },
            integrations: IntegrationsConfig::default(),
            agents: vec![],
        };

        unsafe {
            std::env::set_var("STIGNORE_PORT", "8080");
            std::env::set_var("STIGNORE_MINIMUM_COPIES", "3");
            std::env::set_var("STIGNORE_AGENT_TIMEOUT_SECONDS", "10");
            std::env::set_var("STIGNORE_AUTH_ENABLED", "true");
            std::env::set_var("STIGNORE_AUTH_USER_HEADER", "X-Custom-User");
            std::env::set_var("STIGNORE_AUTH_ROLE_HEADER", "X-Custom-Roles");
            std::env::set_var("STIGNORE_AUTH_ADMIN_ROLE", "SuperAdmin");
            std::env::set_var("STIGNORE_AUTH_READER_ROLE", "Viewer");
        }

        let overridden = apply_manager_env_overrides(base);

        assert_eq!(overridden.manager.port, 8080);
        assert_eq!(overridden.manager.minimum_copies, 3);
        assert_eq!(overridden.manager.agent_timeout_seconds, 10);
        assert!(overridden.manager.auth.enabled);
        assert_eq!(overridden.manager.auth.user_header, "X-Custom-User");
        assert_eq!(overridden.manager.auth.role_header, "X-Custom-Roles");
        assert_eq!(overridden.manager.auth.admin_role, "SuperAdmin");
        assert_eq!(overridden.manager.auth.reader_role, "Viewer");

        unsafe {
            std::env::remove_var("STIGNORE_PORT");
            std::env::remove_var("STIGNORE_MINIMUM_COPIES");
            std::env::remove_var("STIGNORE_AGENT_TIMEOUT_SECONDS");
            std::env::remove_var("STIGNORE_AUTH_ENABLED");
            std::env::remove_var("STIGNORE_AUTH_USER_HEADER");
            std::env::remove_var("STIGNORE_AUTH_ROLE_HEADER");
            std::env::remove_var("STIGNORE_AUTH_ADMIN_ROLE");
            std::env::remove_var("STIGNORE_AUTH_READER_ROLE");
        }
    }

    #[test]
    fn test_manager_config_with_integrations_serde() {
        let toml_str = r#"
            [manager]
            port = 8000
            minimum_copies = 2

            [integrations.radarr]
            enabled = true
            url = "http://localhost:7878"
            api_key = "test-radarr-key"
            category_id = "movies"
            delete_files = false
            add_import_exclusion = true

            [integrations.sonarr]
            enabled = true
            url = "http://localhost:8989"
            api_key = "test-sonarr-key"
            category_id = "tv"
            delete_files = false
            add_import_list_exclusion = false

            [[agents]]
            name = "Agent 1"
            hostname = "localhost:3000"
            api_key = "550e8400-e29b-41d4-a716-446655440000"
        "#;

        let data: Result<ManagerData, toml::de::Error> = toml::from_str(toml_str);
        assert!(data.is_ok());
        let config = data.unwrap();

        let radarr = config.integrations.radarr.unwrap();
        assert!(radarr.enabled);
        assert_eq!(radarr.url, "http://localhost:7878");
        assert_eq!(radarr.api_key, "test-radarr-key");
        assert_eq!(radarr.category_id, "movies");
        assert!(radarr.add_import_exclusion);

        let sonarr = config.integrations.sonarr.unwrap();
        assert!(sonarr.enabled);
        assert_eq!(sonarr.url, "http://localhost:8989");
        assert_eq!(sonarr.api_key, "test-sonarr-key");
        assert_eq!(sonarr.category_id, "tv");
        assert!(!sonarr.add_import_list_exclusion);
    }

    #[test]
    fn test_integrations_env_overrides() {
        let base = ManagerData {
            manager: ManagerConfig {
                port: 8000,
                minimum_copies: 2,
                agent_timeout_seconds: 5,
                auth: AuthConfig::default(),
            },
            integrations: IntegrationsConfig::default(),
            agents: vec![],
        };

        unsafe {
            std::env::set_var("STIGNORE_RADARR_URL", "http://radarr.local:7878");
            std::env::set_var("STIGNORE_RADARR_API_KEY", "radarr-secret");
            std::env::set_var("STIGNORE_RADARR_CATEGORY", "films");
            std::env::set_var("STIGNORE_RADARR_ADD_IMPORT_EXCLUSION", "true");

            std::env::set_var("STIGNORE_SONARR_URL", "http://sonarr.local:8989");
            std::env::set_var("STIGNORE_SONARR_API_KEY", "sonarr-secret");
            std::env::set_var("STIGNORE_SONARR_CATEGORY", "shows");
            std::env::set_var("STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION", "true");
        }

        let overridden = apply_manager_env_overrides(base);

        let radarr = overridden.integrations.radarr.unwrap();
        assert_eq!(radarr.url, "http://radarr.local:7878");
        assert_eq!(radarr.api_key, "radarr-secret");
        assert_eq!(radarr.category_id, "films");
        assert!(radarr.add_import_exclusion);

        let sonarr = overridden.integrations.sonarr.unwrap();
        assert_eq!(sonarr.url, "http://sonarr.local:8989");
        assert_eq!(sonarr.api_key, "sonarr-secret");
        assert_eq!(sonarr.category_id, "shows");
        assert!(sonarr.add_import_list_exclusion);

        unsafe {
            std::env::remove_var("STIGNORE_RADARR_URL");
            std::env::remove_var("STIGNORE_RADARR_API_KEY");
            std::env::remove_var("STIGNORE_RADARR_CATEGORY");
            std::env::remove_var("STIGNORE_RADARR_ADD_IMPORT_EXCLUSION");

            std::env::remove_var("STIGNORE_SONARR_URL");
            std::env::remove_var("STIGNORE_SONARR_API_KEY");
            std::env::remove_var("STIGNORE_SONARR_CATEGORY");
            std::env::remove_var("STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION");
        }
    }

    #[test]
    fn test_resolve_config_path_explicit() {
        let res = resolve_config_path(Some("/path/to/explicit.toml"), &["default.toml"]);
        assert_eq!(res, Some("/path/to/explicit.toml".to_string()));
    }

    #[test]
    fn test_resolve_config_path_env_var() {
        unsafe {
            std::env::set_var("STIGNORE_CONFIG", "/env/config.toml");
        }

        let res = resolve_config_path(None, &["nonexistent.toml"]);
        assert_eq!(res, Some("/env/config.toml".to_string()));

        unsafe {
            std::env::remove_var("STIGNORE_CONFIG");
        }
    }
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionMode {
    Default,
    Protected,
    Hardened,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub auth: AuthConfig,
    pub server: ServerConfig,
    #[serde(default)]
    pub protection: ProtectionConfig,
    #[serde(default)]
    pub file_watch: FileWatchConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub data_dir: String,
    #[serde(default = "default_max_storage_mb")]
    pub max_storage_mb: u64,
}

fn default_max_storage_mb() -> u64 {
    100 // 100MB default
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtectionConfig {
    #[serde(default)]
    pub append_only: bool,
    #[serde(default)]
    pub remote_syslog: Option<RemoteSyslogConfig>,
    #[serde(default)]
    pub sign_events: bool,
    #[serde(default)]
    pub signing_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteSyslogConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub protocol: String, // "tcp" or "udp"
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileWatchConfig {
    pub enabled: bool,
    pub watch_dirs: Vec<String>,
}

impl Default for FileWatchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            watch_dirs: vec![],
        }
    }
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            append_only: false,
            remote_syslog: None,
            sign_events: false,
            signing_key: None,
        }
    }
}

impl Config {
    // Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let config_path = "./config.toml";

        if !std::path::Path::new(config_path).exists() {
            return Self::create_default();
        }

        let content = fs::read_to_string(config_path).context("Failed to read config.toml")?;
        let config: Config = toml::from_str(&content).context("Failed to parse config.toml")?;
        Ok(config)
    }

    // Create default config with admin/admin credentials
    fn create_default() -> Result<Self> {
        println!("Config file not found. Creating default config.toml...");

        // Generate bcrypt hash for default password "admin"
        let default_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST)
            .context("Failed to generate default password hash")?;

        let config = Config {
            auth: AuthConfig {
                enabled: true,
                username: "admin".to_string(),
                password_hash: default_hash,
            },
            server: ServerConfig {
                port: 8080,
                data_dir: "./data".to_string(),
                max_storage_mb: 100,
            },
            protection: ProtectionConfig::default(),
            file_watch: FileWatchConfig::default(),
        };

        let toml_content = toml::to_string_pretty(&config)
            .context("Failed to serialize default config")?;

        fs::write("./config.toml", toml_content).context("Failed to write config.toml")?;

        println!("\nSECURITY WARNING");
        println!("Created config.toml with default credentials:");
        println!("  Username: admin");
        println!("  Password: admin");
        println!("\nPLEASE CHANGE THE DEFAULT PASSWORD IMMEDIATELY!");
        println!("Run: cargo run --bin hashpw <your-password>");
        println!("Then update the password_hash in config.toml\n");

        Ok(config)
    }

    // Create a test config (for unit tests)
    #[cfg(test)]
    pub fn test_config() -> Self {
        Config {
            auth: AuthConfig {
                enabled: true,
                username: "test".to_string(),
                password_hash: bcrypt::hash("test", 4).unwrap(),
            },
            server: ServerConfig {
                port: 8080,
                data_dir: "./test_data".to_string(),
                max_storage_mb: 100,
            },
            protection: ProtectionConfig::default(),
            file_watch: FileWatchConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = Config::test_config();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("username"));
        assert!(toml_str.contains("password_hash"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [auth]
            enabled = true
            username = "admin"
            password_hash = "$2b$12$test"

            [server]
            port = 8080
            data_dir = "./data"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.auth.username, "admin");
        assert_eq!(config.server.port, 8080);
    }
}

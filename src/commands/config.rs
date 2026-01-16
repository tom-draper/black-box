use anyhow::{Context, Result};
use std::fs;

use crate::config::{Config, RemoteSyslogConfig};

pub fn show_config() -> Result<()> {
    let config = Config::load()?;
    let toml_content = toml::to_string_pretty(&config)
        .context("Failed to serialize config")?;

    println!("Current Configuration");
    println!("=====================");
    println!();
    println!("{}", toml_content);

    Ok(())
}

pub fn validate_config() -> Result<()> {
    println!("Validating config.toml...");

    match Config::load() {
        Ok(config) => {
            println!("✓ Configuration is valid");
            println!();
            println!("Server:");
            println!("  Port: {}", config.server.port);
            println!("  Data directory: {}", config.server.data_dir);
            println!();
            println!("Authentication:");
            println!("  Enabled: {}", config.auth.enabled);
            if config.auth.enabled {
                println!("  Username: {}", config.auth.username);
                println!("  Password hash: {}...", &config.auth.password_hash[..20]);
            }
            println!();
            println!("Protection:");
            println!("  Append-only: {}", config.protection.append_only);
            println!("  Sign events: {}", config.protection.sign_events);
            if let Some(ref syslog) = config.protection.remote_syslog {
                println!("  Remote syslog: {} ({}:{})",
                    if syslog.enabled { "enabled" } else { "disabled" },
                    syslog.host,
                    syslog.port
                );
            } else {
                println!("  Remote syslog: not configured");
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Configuration is invalid:");
            eprintln!("  {}", e);
            std::process::exit(1);
        }
    }
}

pub fn init_config(force: bool) -> Result<()> {
    let config_path = "./config.toml";

    if std::path::Path::new(config_path).exists() && !force {
        anyhow::bail!(
            "Config file already exists at {}. Use --force to overwrite.",
            config_path
        );
    }

    println!("Generating default configuration...");

    // Create default config
    let config = Config::load()?;

    let toml_content = toml::to_string_pretty(&config)
        .context("Failed to serialize config")?;

    fs::write(config_path, toml_content)
        .context("Failed to write config file")?;

    println!("✓ Default configuration written to {}", config_path);
    println!();
    println!("SECURITY WARNING");
    println!("================");
    println!("Default credentials:");
    println!("  Username: admin");
    println!("  Password: admin");
    println!();
    println!("PLEASE CHANGE THE DEFAULT PASSWORD IMMEDIATELY!");
    println!();
    println!("To generate a password hash:");
    println!("  Run: echo -n 'your-password' | bcrypt");
    println!("  Or use an online bcrypt generator");
    println!("  Then update the password_hash in config.toml");

    Ok(())
}

pub fn setup_remote_syslog(host: String, port: u16, protocol: String) -> Result<()> {
    let config_path = "./config.toml";

    // Load existing config
    let mut config = if std::path::Path::new(config_path).exists() {
        let content = fs::read_to_string(config_path)
            .context("Failed to read config.toml")?;
        toml::from_str(&content).context("Failed to parse config.toml")?
    } else {
        println!("Config file not found, creating new one...");
        Config::load()?
    };

    // Validate protocol
    if protocol != "tcp" && protocol != "udp" {
        anyhow::bail!("Protocol must be 'tcp' or 'udp', got '{}'", protocol);
    }

    // Update remote syslog config
    config.protection.remote_syslog = Some(RemoteSyslogConfig {
        enabled: true,
        host: host.clone(),
        port,
        protocol: protocol.clone(),
    });

    // Save config
    let toml_content = toml::to_string_pretty(&config)
        .context("Failed to serialize config")?;

    fs::write(config_path, toml_content)
        .context("Failed to write config file")?;

    println!("✓ Remote syslog configured");
    println!();
    println!("Configuration:");
    println!("  Host: {}", host);
    println!("  Port: {}", port);
    println!("  Protocol: {}", protocol);
    println!();
    println!("Remote syslog streaming will be enabled when running in");
    println!("--protected or --hardened mode.");
    println!();
    println!("To test the connection:");
    println!("  1. Start your black box: black-box run --protected");
    println!("  2. Check the logs for connection status");
    println!("  3. Verify events are being received on {}:{}", host, port);

    Ok(())
}

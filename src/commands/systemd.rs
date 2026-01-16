use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub fn generate_service(
    binary_path: String,
    working_dir: String,
    data_dir: String,
    export_on_stop: bool,
    export_dir: String,
    output: Option<String>,
) -> Result<()> {
    let service_content = generate_service_content(
        &binary_path,
        &working_dir,
        &data_dir,
        export_on_stop,
        &export_dir,
    );

    if let Some(output_path) = output {
        fs::write(&output_path, service_content)
            .context("Failed to write service file")?;
        println!("Systemd service file written to: {}", output_path);
        println!();
        println!("To install:");
        println!("  sudo cp {} /etc/systemd/system/black-box.service", output_path);
        println!("  sudo systemctl daemon-reload");
        println!("  sudo systemctl enable black-box.service");
        println!("  sudo systemctl start black-box.service");
    } else {
        println!("{}", service_content);
    }

    Ok(())
}

pub fn install_service(
    binary_path: String,
    working_dir: String,
    export_on_stop: bool,
) -> Result<()> {
    // Check if running as root
    if unsafe { libc::geteuid() } != 0 {
        anyhow::bail!("Installation requires root privileges. Run with sudo.");
    }

    println!("Installing Black Box as systemd service...");
    println!();

    // Create working directory
    fs::create_dir_all(&working_dir)
        .context("Failed to create working directory")?;

    let data_dir = format!("{}/data", working_dir);
    fs::create_dir_all(&data_dir)
        .context("Failed to create data directory")?;

    let export_dir = "/var/backups/black-box";
    if export_on_stop {
        fs::create_dir_all(export_dir)
            .context("Failed to create export directory")?;
    }

    // Generate service content
    let service_content = generate_service_content(
        &binary_path,
        &working_dir,
        &data_dir,
        export_on_stop,
        export_dir,
    );

    // Write service file
    let service_path = "/etc/systemd/system/black-box.service";
    fs::write(service_path, service_content)
        .context("Failed to write service file")?;

    println!("✓ Service file written to {}", service_path);

    // Copy binary if it doesn't exist at target
    if !std::path::Path::new(&binary_path).exists() {
        let current_exe = std::env::current_exe()
            .context("Failed to get current executable path")?;
        fs::copy(&current_exe, &binary_path)
            .context("Failed to copy binary")?;
        println!("✓ Binary copied to {}", binary_path);

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms)?;
        }
    }

    // Create default config in working directory
    let config_path = format!("{}/config.toml", working_dir);
    if !std::path::Path::new(&config_path).exists() {
        // Generate default config
        let config_content = generate_default_config(&data_dir);
        fs::write(&config_path, config_content)
            .context("Failed to write config file")?;
        println!("✓ Default config written to {}", config_path);
        println!("  WARNING: Using default credentials (admin/admin)");
        println!("  Please update the password in {}", config_path);
    }

    // Reload systemd
    println!();
    println!("Reloading systemd...");
    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .context("Failed to reload systemd")?;

    println!("✓ Systemd reloaded");
    println!();
    println!("Installation complete!");
    println!();
    println!("To start the service:");
    println!("  sudo systemctl start black-box");
    println!();
    println!("To enable on boot:");
    println!("  sudo systemctl enable black-box");
    println!();
    println!("To check status:");
    println!("  sudo systemctl status black-box");
    println!("  black-box status");

    Ok(())
}

fn generate_service_content(
    binary_path: &str,
    working_dir: &str,
    data_dir: &str,
    export_on_stop: bool,
    export_dir: &str,
) -> String {
    let exec_stop_post = if export_on_stop {
        format!(
            "ExecStopPost={} export --data-dir {} --output {}/emergency-export-$(date +%%Y%%m%%d-%%H%%M%%S).json.gz --compress\n",
            binary_path, data_dir, export_dir
        )
    } else {
        String::new()
    };

    format!(
        r#"[Unit]
Description=Black Box - Tamper-Resistant Server Event Recorder
After=network.target
Documentation=https://github.com/yourusername/black-box

[Service]
Type=simple
ExecStart={binary_path} run --protected
WorkingDirectory={working_dir}
Restart=always
RestartSec=5s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=black-box

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths={data_dir}
ReadWritePaths={working_dir}
{export_dir_rw}
# Auto-export on service stop (emergency backup)
{exec_stop_post}
# Graceful shutdown
TimeoutStopSec=30s
KillMode=mixed
KillSignal=SIGTERM

[Install]
WantedBy=multi-user.target
"#,
        binary_path = binary_path,
        working_dir = working_dir,
        data_dir = data_dir,
        export_dir_rw = if export_on_stop {
            format!("ReadWritePaths={}", export_dir)
        } else {
            String::new()
        },
        exec_stop_post = exec_stop_post,
    )
}

fn generate_default_config(data_dir: &str) -> String {
    format!(
        r#"[auth]
enabled = true
username = "admin"
# Default password: "admin" - CHANGE THIS!
password_hash = "$2b$12$KIXALxKzLbXHQXQZWxJQfOqK.vlWvPXPvvPZvqKq3vKZvXvXvXvXe"

[server]
port = 8080
data_dir = "{}"

[protection]
append_only = true

[protection.remote_syslog]
enabled = false
host = "syslog.example.com"
port = 514
protocol = "tcp"
"#,
        data_dir
    )
}

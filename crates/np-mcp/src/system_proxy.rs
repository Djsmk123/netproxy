//! macOS system proxy configuration via networksetup.
//!
//! Enables AI to configure system-wide HTTP/HTTPS proxy settings.

use serde_json::{json, Value};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemProxyError {
    #[error("failed to execute networksetup: {0}")]
    CommandFailed(String),
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Get the primary network interface (usually Wi-Fi or Ethernet).
fn get_primary_interface() -> Result<String, SystemProxyError> {
    let output = Command::new("networksetup")
        .args(&["-listallnetworkservices"])
        .output()
        .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(SystemProxyError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let services = String::from_utf8_lossy(&output.stdout);
    // First non-asterisk line is typically Wi-Fi or Ethernet
    for line in services.lines().skip(1) {
        let service = line.trim();
        if !service.is_empty() && !service.starts_with('*') {
            return Ok(service.to_string());
        }
    }

    Err(SystemProxyError::ParseError(
        "no active network service found".to_string(),
    ))
}

/// Set system proxy for HTTP and HTTPS.
pub fn set_system_proxy(
    host: &str,
    port: u16,
    interface: Option<&str>,
) -> Result<Value, SystemProxyError> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(SystemProxyError::UnsupportedPlatform(
            "system proxy control is only supported on macOS".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let interface = match interface {
            Some(i) => i.to_string(),
            None => get_primary_interface()?,
        };

        // Set web proxy (HTTP)
        let http_output = Command::new("networksetup")
            .args(&["-setwebproxy", &interface, host, &port.to_string()])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        if !http_output.status.success() {
            return Err(SystemProxyError::CommandFailed(
                String::from_utf8_lossy(&http_output.stderr).to_string(),
            ));
        }

        // Set secure web proxy (HTTPS)
        let https_output = Command::new("networksetup")
            .args(&["-setsecurewebproxy", &interface, host, &port.to_string()])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        if !https_output.status.success() {
            return Err(SystemProxyError::CommandFailed(
                String::from_utf8_lossy(&https_output.stderr).to_string(),
            ));
        }

        // Enable the proxies
        Command::new("networksetup")
            .args(&["-setwebproxystate", &interface, "on"])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        Command::new("networksetup")
            .args(&["-setsecurewebproxystate", &interface, "on"])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        Ok(json!({
            "success": true,
            "interface": interface,
            "http_proxy": format!("{}:{}", host, port),
            "https_proxy": format!("{}:{}", host, port)
        }))
    }
}

/// Clear system proxy settings.
pub fn clear_system_proxy(interface: Option<&str>) -> Result<Value, SystemProxyError> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(SystemProxyError::UnsupportedPlatform(
            "system proxy control is only supported on macOS".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let interface = match interface {
            Some(i) => i.to_string(),
            None => get_primary_interface()?,
        };

        Command::new("networksetup")
            .args(&["-setwebproxystate", &interface, "off"])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        Command::new("networksetup")
            .args(&["-setsecurewebproxystate", &interface, "off"])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        Ok(json!({
            "success": true,
            "interface": interface,
            "cleared": true
        }))
    }
}

/// Get current system proxy status.
pub fn get_system_proxy_status(interface: Option<&str>) -> Result<Value, SystemProxyError> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(SystemProxyError::UnsupportedPlatform(
            "system proxy control is only supported on macOS".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let interface = match interface {
            Some(i) => i.to_string(),
            None => get_primary_interface()?,
        };

        let http_output = Command::new("networksetup")
            .args(&["-getwebproxy", &interface])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        let https_output = Command::new("networksetup")
            .args(&["-getsecurewebproxy", &interface])
            .output()
            .map_err(|e| SystemProxyError::CommandFailed(e.to_string()))?;

        let http_info = String::from_utf8_lossy(&http_output.stdout);
        let https_info = String::from_utf8_lossy(&https_output.stdout);

        Ok(json!({
            "interface": interface,
            "http": parse_proxy_info(&http_info),
            "https": parse_proxy_info(&https_info)
        }))
    }
}

#[cfg(target_os = "macos")]
fn parse_proxy_info(info: &str) -> Value {
    let mut enabled = false;
    let mut server = String::new();
    let mut port = 0u16;

    for line in info.lines() {
        if line.contains("Enabled: Yes") {
            enabled = true;
        } else if line.starts_with("Server:") {
            server = line.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("Port:") {
            port = line
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }

    json!({
        "enabled": enabled,
        "server": server,
        "port": port
    })
}

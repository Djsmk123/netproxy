//! Android Debug Bridge (ADB) proxy configuration.
//!
//! Enables AI to configure proxy settings on Android devices and emulators.

use serde_json::{json, Value};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdbError {
    #[error("adb command failed: {0}")]
    CommandFailed(String),
    #[error("adb not found: install Android SDK platform-tools")]
    AdbNotFound,
    #[error("no devices connected")]
    NoDevices,
    #[error("device not found: {0}")]
    DeviceNotFound(String),
}

/// Check if adb is available.
fn check_adb() -> Result<(), AdbError> {
    Command::new("adb")
        .arg("version")
        .output()
        .map_err(|_| AdbError::AdbNotFound)?;
    Ok(())
}

/// List connected Android devices.
pub fn list_devices() -> Result<Value, AdbError> {
    check_adb()?;

    let output = Command::new("adb")
        .arg("devices")
        .output()
        .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(AdbError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let devices_str = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in devices_str.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "device" {
            devices.push(json!({
                "id": parts[0],
                "state": parts[1]
            }));
        }
    }

    if devices.is_empty() {
        return Err(AdbError::NoDevices);
    }

    Ok(json!({
        "devices": devices,
        "count": devices.len()
    }))
}

/// Set proxy on Android device via ADB.
pub fn set_proxy(device: Option<&str>, host: &str, port: u16) -> Result<Value, AdbError> {
    check_adb()?;

    let device_id = match device {
        Some(d) => d.to_string(),
        None => {
            // Use first available device
            let devices = list_devices()?;
            devices["devices"][0]["id"]
                .as_str()
                .ok_or(AdbError::NoDevices)?
                .to_string()
        }
    };

    // Set global HTTP proxy
    let proxy_str = format!("{}:{}", host, port);
    let output = Command::new("adb")
        .args(&[
            "-s",
            &device_id,
            "shell",
            "settings",
            "put",
            "global",
            "http_proxy",
            &proxy_str,
        ])
        .output()
        .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(AdbError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(json!({
        "success": true,
        "device": device_id,
        "proxy": proxy_str
    }))
}

/// Clear proxy from Android device.
pub fn clear_proxy(device: Option<&str>) -> Result<Value, AdbError> {
    check_adb()?;

    let device_id = match device {
        Some(d) => d.to_string(),
        None => {
            let devices = list_devices()?;
            devices["devices"][0]["id"]
                .as_str()
                .ok_or(AdbError::NoDevices)?
                .to_string()
        }
    };

    // Clear by setting to :0
    let output = Command::new("adb")
        .args(&[
            "-s",
            &device_id,
            "shell",
            "settings",
            "put",
            "global",
            "http_proxy",
            ":0",
        ])
        .output()
        .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(AdbError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(json!({
        "success": true,
        "device": device_id,
        "cleared": true
    }))
}

/// Get proxy status from Android device.
pub fn get_proxy_status(device: Option<&str>) -> Result<Value, AdbError> {
    check_adb()?;

    let device_id = match device {
        Some(d) => d.to_string(),
        None => {
            let devices = list_devices()?;
            devices["devices"][0]["id"]
                .as_str()
                .ok_or(AdbError::NoDevices)?
                .to_string()
        }
    };

    let output = Command::new("adb")
        .args(&[
            "-s",
            &device_id,
            "shell",
            "settings",
            "get",
            "global",
            "http_proxy",
        ])
        .output()
        .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

    let proxy_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let enabled = !proxy_str.is_empty() && proxy_str != ":0";

    Ok(json!({
        "device": device_id,
        "enabled": enabled,
        "proxy": if enabled { proxy_str } else { String::new() }
    }))
}

/// Install NetProxy CA certificate on Android device.
pub fn install_ca_cert(device: Option<&str>, cert_path: &str) -> Result<Value, AdbError> {
    check_adb()?;

    let device_id = match device {
        Some(d) => d.to_string(),
        None => {
            let devices = list_devices()?;
            devices["devices"][0]["id"]
                .as_str()
                .ok_or(AdbError::NoDevices)?
                .to_string()
        }
    };

    // Push certificate to device
    let push_output = Command::new("adb")
        .args(&[
            "-s",
            &device_id,
            "push",
            cert_path,
            "/sdcard/netproxy-ca.crt",
        ])
        .output()
        .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

    if !push_output.status.success() {
        return Err(AdbError::CommandFailed(
            String::from_utf8_lossy(&push_output.stderr).to_string(),
        ));
    }

    Ok(json!({
        "success": true,
        "device": device_id,
        "message": "Certificate pushed to /sdcard/netproxy-ca.crt. Install manually from Settings > Security > Install from storage"
    }))
}

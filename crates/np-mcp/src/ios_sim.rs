//! iOS Simulator proxy configuration via simctl.
//!
//! Enables AI to configure proxy settings on iOS simulators.

use serde_json::{json, Value};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IosSimError {
    #[error("simctl command failed: {0}")]
    CommandFailed(String),
    #[error("xcrun not found: Xcode or Command Line Tools required")]
    XcrunNotFound,
    #[error("no simulators available")]
    NoSimulators,
    #[error("simulator not found: {0}")]
    SimulatorNotFound(String),
}

/// Check if xcrun is available.
fn check_xcrun() -> Result<(), IosSimError> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(IosSimError::XcrunNotFound);
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("xcrun")
            .args(&["simctl", "help"])
            .output()
            .map_err(|_| IosSimError::XcrunNotFound)?;
        Ok(())
    }
}

/// List available iOS simulators.
pub fn list_simulators() -> Result<Value, IosSimError> {
    check_xcrun()?;

    let output = Command::new("xcrun")
        .args(&["simctl", "list", "devices", "available", "--json"])
        .output()
        .map_err(|e| IosSimError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(IosSimError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let devices_json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| IosSimError::CommandFailed(e.to_string()))?;

    let mut simulators = Vec::new();

    if let Some(devices) = devices_json["devices"].as_object() {
        for (runtime, device_list) in devices {
            if let Some(devices) = device_list.as_array() {
                for device in devices {
                    if device["isAvailable"].as_bool().unwrap_or(false) {
                        simulators.push(json!({
                            "name": device["name"],
                            "udid": device["udid"],
                            "state": device["state"],
                            "runtime": runtime
                        }));
                    }
                }
            }
        }
    }

    Ok(json!({
        "simulators": simulators,
        "count": simulators.len()
    }))
}

/// Set proxy on iOS Simulator.
///
/// Note: iOS Simulator proxy configuration is complex and may require
/// manually editing network settings in the simulator's Settings app.
/// This function provides guidance.
pub fn set_proxy(_udid: Option<&str>, _host: &str, _port: u16) -> Result<Value, IosSimError> {
    check_xcrun()?;

    // iOS Simulator doesn't have a direct command-line API for proxy settings
    // Users need to configure it manually through Settings app
    Ok(json!({
        "success": false,
        "message": "iOS Simulator proxy must be configured manually",
        "instructions": [
            "1. Open Simulator",
            "2. Go to Settings > Wi-Fi",
            "3. Tap the (i) icon next to the connected network",
            "4. Scroll to HTTP Proxy section",
            "5. Select Manual and enter proxy details"
        ]
    }))
}

/// Clear proxy from iOS Simulator.
pub fn clear_proxy(_udid: Option<&str>) -> Result<Value, IosSimError> {
    check_xcrun()?;

    Ok(json!({
        "success": false,
        "message": "iOS Simulator proxy must be cleared manually",
        "instructions": [
            "1. Open Simulator",
            "2. Go to Settings > Wi-Fi",
            "3. Tap the (i) icon next to the connected network",
            "4. Scroll to HTTP Proxy section",
            "5. Select Off"
        ]
    }))
}

/// Install CA certificate in iOS Simulator.
pub fn install_ca_cert(udid: Option<&str>, cert_path: &str) -> Result<Value, IosSimError> {
    check_xcrun()?;

    let sim_udid = match udid {
        Some(u) => u.to_string(),
        None => {
            // Get booted simulator
            let output = Command::new("xcrun")
                .args(&["simctl", "list", "devices", "booted", "--json"])
                .output()
                .map_err(|e| IosSimError::CommandFailed(e.to_string()))?;

            let booted: Value = serde_json::from_slice(&output.stdout)
                .map_err(|e| IosSimError::CommandFailed(e.to_string()))?;

            // Find first booted device
            let mut found_udid = None;
            if let Some(devices) = booted["devices"].as_object() {
                for (_, device_list) in devices {
                    if let Some(devices) = device_list.as_array() {
                        for device in devices {
                            if device["state"] == "Booted" {
                                found_udid = device["udid"].as_str().map(|s| s.to_string());
                                break;
                            }
                        }
                    }
                    if found_udid.is_some() {
                        break;
                    }
                }
            }

            found_udid.ok_or(IosSimError::NoSimulators)?
        }
    };

    // Open certificate in simulator (opens in Safari, user installs from there)
    let output = Command::new("xcrun")
        .args(&["simctl", "openurl", &sim_udid, &format!("file://{}", cert_path)])
        .output()
        .map_err(|e| IosSimError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(IosSimError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(json!({
        "success": true,
        "simulator": sim_udid,
        "message": "Certificate opened in simulator. Install from Settings > General > VPN & Device Management > Profile"
    }))
}

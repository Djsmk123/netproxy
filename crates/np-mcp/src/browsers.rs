//! Browser launching with proxy configuration.
//!
//! Enables AI to launch browsers with NetProxy configured.

use serde_json::{json, Value};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("browser launch failed: {0}")]
    LaunchFailed(String),
    #[error("browser not found: {0}")]
    BrowserNotFound(String),
    #[error("unsupported browser: {0}")]
    UnsupportedBrowser(String),
}

#[derive(Debug, Clone, Copy)]
pub enum Browser {
    Chrome,
    Firefox,
    Safari,
}

impl Browser {
    fn from_str(s: &str) -> Result<Self, BrowserError> {
        match s.to_lowercase().as_str() {
            "chrome" | "google chrome" => Ok(Browser::Chrome),
            "firefox" => Ok(Browser::Firefox),
            "safari" => Ok(Browser::Safari),
            _ => Err(BrowserError::UnsupportedBrowser(s.to_string())),
        }
    }

    #[cfg(target_os = "macos")]
    fn app_path(&self) -> &'static str {
        match self {
            Browser::Chrome => "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            Browser::Firefox => "/Applications/Firefox.app/Contents/MacOS/firefox",
            Browser::Safari => "/Applications/Safari.app/Contents/MacOS/Safari",
        }
    }

    #[cfg(target_os = "linux")]
    fn app_path(&self) -> &'static str {
        match self {
            Browser::Chrome => "google-chrome",
            Browser::Firefox => "firefox",
            Browser::Safari => {
                // Safari doesn't exist on Linux
                ""
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn app_path(&self) -> &'static str {
        match self {
            Browser::Chrome => "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            Browser::Firefox => "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
            Browser::Safari => "",
        }
    }
}

/// Launch browser with proxy configuration.
pub fn launch_with_proxy(
    browser: &str,
    host: &str,
    port: u16,
    ignore_cert_errors: bool,
    user_data_dir: Option<&str>,
) -> Result<Value, BrowserError> {
    let browser_type = Browser::from_str(browser)?;
    let proxy_str = format!("{}:{}", host, port);

    match browser_type {
        Browser::Chrome => launch_chrome(&proxy_str, ignore_cert_errors, user_data_dir),
        Browser::Firefox => launch_firefox(&proxy_str, ignore_cert_errors, user_data_dir),
        Browser::Safari => Err(BrowserError::UnsupportedBrowser(
            "Safari doesn't support command-line proxy configuration".to_string(),
        )),
    }
}

fn launch_chrome(
    proxy: &str,
    ignore_cert_errors: bool,
    user_data_dir: Option<&str>,
) -> Result<Value, BrowserError> {
    let data_dir = user_data_dir.unwrap_or("/tmp/chrome-netproxy");
    let app_path = Browser::Chrome.app_path();

    // Check if Chrome exists
    if !std::path::Path::new(app_path).exists() {
        #[cfg(target_os = "linux")]
        {
            // On Linux, try which command
            let which_output = Command::new("which")
                .arg("google-chrome")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });

            if which_output.is_none() {
                return Err(BrowserError::BrowserNotFound("Chrome".to_string()));
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            return Err(BrowserError::BrowserNotFound("Chrome".to_string()));
        }
    }

    let mut args = vec![
        format!("--proxy-server={}", proxy),
        format!("--user-data-dir={}", data_dir),
    ];

    if ignore_cert_errors {
        args.push("--ignore-certificate-errors".to_string());
    }

    let mut cmd = Command::new(app_path);
    cmd.args(&args);

    #[cfg(unix)]
    {
        // Detach from current process
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()
        .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

    Ok(json!({
        "success": true,
        "browser": "Chrome",
        "proxy": proxy,
        "user_data_dir": data_dir,
        "ignore_cert_errors": ignore_cert_errors
    }))
}

fn launch_firefox(
    _proxy: &str,
    _ignore_cert_errors: bool,
    _user_data_dir: Option<&str>,
) -> Result<Value, BrowserError> {
    // Firefox proxy configuration is more complex (requires profile modification)
    Err(BrowserError::UnsupportedBrowser(
        "Firefox proxy configuration via CLI is not yet implemented".to_string(),
    ))
}

/// Close browser instances.
pub fn close_browser(browser: &str) -> Result<Value, BrowserError> {
    let browser_type = Browser::from_str(browser)?;

    #[cfg(target_os = "macos")]
    {
        let app_name = match browser_type {
            Browser::Chrome => "Google Chrome",
            Browser::Firefox => "Firefox",
            Browser::Safari => "Safari",
        };

        Command::new("killall")
            .arg(app_name)
            .output()
            .ok();

        return Ok(json!({
            "success": true,
            "browser": app_name
        }));
    }

    #[cfg(target_os = "linux")]
    {
        let process_name = match browser_type {
            Browser::Chrome => "chrome",
            Browser::Firefox => "firefox",
            Browser::Safari => return Err(BrowserError::BrowserNotFound("Safari".to_string())),
        };

        Command::new("pkill")
            .arg(process_name)
            .output()
            .ok();

        return Ok(json!({
            "success": true,
            "browser": process_name
        }));
    }

    #[cfg(target_os = "windows")]
    {
        let process_name = match browser_type {
            Browser::Chrome => "chrome.exe",
            Browser::Firefox => "firefox.exe",
            Browser::Safari => return Err(BrowserError::BrowserNotFound("Safari".to_string())),
        };

        Command::new("taskkill")
            .args(&["/F", "/IM", process_name])
            .output()
            .ok();

        return Ok(json!({
            "success": true,
            "browser": process_name
        }));
    }
}

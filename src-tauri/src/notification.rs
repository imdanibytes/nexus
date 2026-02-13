//! Cross-platform native OS notifications via notify-rust.
//!
//! - macOS: NSUserNotificationCenter (via mac-notification-sys)
//! - Windows: WinRT toast notifications
//! - Linux: freedesktop D-Bus notifications

/// Set the application identity for notifications. Call once at app startup.
pub fn init() {
    #[cfg(target_os = "macos")]
    {
        // Tell mac-notification-sys to send notifications as our bundle ID
        // so they appear under "Nexus" in Notification Center, not "Terminal".
        let bundle_id = "com.nexus-dashboard.desktop";
        if let Err(e) = notify_rust::set_application(bundle_id) {
            log::warn!("[notification] failed to set application identity: {e}");
        }
    }
}

#[tauri::command]
pub fn send_notification(title: String, body: String) -> Result<(), String> {
    notify_rust::Notification::new()
        .summary(&title)
        .body(&body)
        .show()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

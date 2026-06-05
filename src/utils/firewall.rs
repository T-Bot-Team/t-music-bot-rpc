
pub fn add_firewall_rule(port: u16) {
    #[cfg(target_os = "windows")]
    {
        info!("[Firewall] Windows handles network authorization natively on bind. Skipping elevated PowerShell firewall rule creation for Port {} to avoid security alerts.", port);
    }

    #[cfg(target_os = "macos")]
    {
        info!("[Firewall] macOS handles network authorization natively on bind. Skipping rule creation for Port {} to avoid security alerts.", port);
    }

    #[cfg(target_os = "linux")]
    {
        info!("[Firewall] Linux handles network authorization natively on bind. Skipping elevated pkexec/ufw rule creation for Port {} to avoid security alerts.", port);
    }
}

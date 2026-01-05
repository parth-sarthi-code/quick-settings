use anyhow::Result;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerActionType {
    Shutdown,
    Reboot,
    Logout,
}

/// Power service using systemctl/loginctl commands
#[allow(dead_code)]
pub struct PowerService {}

impl PowerService {
    /// Create power service (uses systemctl, no D-Bus needed)
    #[allow(dead_code)]
    pub fn new_stub() -> Self {
        Self {}
    }

    /// Execute power action
    #[allow(dead_code)]
    pub async fn execute(&self, action: PowerActionType) -> Result<()> {
        match action {
            PowerActionType::Shutdown => self.shutdown().await,
            PowerActionType::Reboot => self.reboot().await,
            PowerActionType::Logout => self.logout().await,
        }
    }

    /// Shutdown the system using systemctl
    #[allow(dead_code)]
    async fn shutdown(&self) -> Result<()> {
        eprintln!("[POWER] Shutdown initiated");
        let output = tokio::process::Command::new("systemctl")
            .arg("poweroff")
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[POWER] ✓ Shutdown command sent");
                Ok(())
            }
            _ => {
                eprintln!("[POWER] ✗ Shutdown failed - may need permissions");
                Err(anyhow::anyhow!("systemctl poweroff failed"))
            }
        }
    }

    /// Reboot the system using systemctl
    #[allow(dead_code)]
    async fn reboot(&self) -> Result<()> {
        eprintln!("[POWER] Reboot initiated");
        let output = tokio::process::Command::new("systemctl")
            .arg("reboot")
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[POWER] ✓ Reboot command sent");
                Ok(())
            }
            _ => {
                eprintln!("[POWER] ✗ Reboot failed - may need permissions");
                Err(anyhow::anyhow!("systemctl reboot failed"))
            }
        }
    }

    /// Logout from session using loginctl
    #[allow(dead_code)]
    async fn logout(&self) -> Result<()> {
        eprintln!("[POWER] Logout initiated");
        let output = tokio::process::Command::new("loginctl")
            .args(["terminate-session", "self"])
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[POWER] ✓ Logout command sent");
                Ok(())
            }
            _ => {
                eprintln!("[POWER] ✗ Logout failed - trying niri exit");
                // Fallback to niri exit
                tokio::process::Command::new("niri")
                    .args(["msg", "action", "quit"])
                    .output()
                    .await
                    .ok();
                Ok(())
            }
        }
    }
}

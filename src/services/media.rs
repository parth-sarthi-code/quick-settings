use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::MediaState;

/// Media service using playerctl (MPRIS command-line tool)
#[allow(dead_code)]
pub struct MediaService {
    current_state: Arc<RwLock<MediaState>>,
}

impl MediaService {
    /// Create media service (uses playerctl, no D-Bus needed)
    #[allow(dead_code)]
    pub fn new_stub() -> Self {
        Self {
            current_state: Arc::new(RwLock::new(MediaState::default())),
        }
    }

    /// Get current media state
    #[allow(dead_code)]
    pub async fn state(&self) -> MediaState {
        self.current_state.read().await.clone()
    }

    /// Play/Pause current track using playerctl
    #[allow(dead_code)]
    pub async fn play_pause(&self) -> Result<()> {
        let output = tokio::process::Command::new("playerctl")
            .arg("play-pause")
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[MEDIA] ✓ Play/Pause toggled");
                Ok(())
            }
            Ok(result) => {
                let err = String::from_utf8_lossy(&result.stderr);
                eprintln!("[MEDIA] ✗ playerctl failed: {}", err);
                Err(anyhow::anyhow!("playerctl command failed"))
            }
            Err(e) => {
                eprintln!("[MEDIA] ✗ playerctl not found or failed: {}", e);
                Err(anyhow::anyhow!(e))
            }
        }
    }

    /// Play next track using playerctl
    #[allow(dead_code)]
    pub async fn next(&self) -> Result<()> {
        let output = tokio::process::Command::new("playerctl")
            .arg("next")
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[MEDIA] ✓ Next track");
                Ok(())
            }
            _ => {
                eprintln!("[MEDIA] ✗ Next track failed");
                Err(anyhow::anyhow!("playerctl command failed"))
            }
        }
    }

    /// Play previous track using playerctl
    #[allow(dead_code)]
    pub async fn previous(&self) -> Result<()> {
        let output = tokio::process::Command::new("playerctl")
            .arg("previous")
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => {
                eprintln!("[MEDIA] ✓ Previous track");
                Ok(())
            }
            _ => {
                eprintln!("[MEDIA] ✗ Previous track failed");
                Err(anyhow::anyhow!("playerctl command failed"))
            }
        }
    }
}

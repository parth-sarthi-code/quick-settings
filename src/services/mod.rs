use serde::{Deserialize, Serialize};
pub mod audio;
pub mod bluetooth;
pub mod device_client;
pub mod deviced;
pub mod media;
pub mod network;
pub mod power;
pub mod runtime;

/// Service states for UI updates
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioState {
    pub volume: f64,
    pub is_muted: bool,
    pub volume_percent: u32,
    pub outputs: Vec<AudioOutput>,
    pub default_output: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOutput {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct MediaState {
    pub is_playing: bool,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub player_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PowerAction {
    pub action_type: power::PowerActionType,
}

// Re-export PowerActionType for convenience
// pub use power::PowerActionType; // currently unused

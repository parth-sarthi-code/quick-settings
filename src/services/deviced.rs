use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default socket path for the device daemon
pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime).join("niri-bar-deviced.sock");
    }
    PathBuf::from("/tmp/niri-bar-deviced.sock")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub name: String,
    pub mac: String,
    pub connected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum DaemonRequest {
    Ping,
    WifiList,
    WifiConnected,
    WifiConnect { ssid: String },
    WifiDisconnect,
    BluetoothList,
    BluetoothConnected,
    BluetoothConnect { mac: String },
    BluetoothDisconnect { mac: String },
    AudioOutputs,
    AudioSetDefault { id: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> DaemonResponse<T> {
    #[allow(dead_code)]
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }

    pub fn into_result(self) -> Result<T> {
        if self.ok {
            self.data.context("missing data in ok response")
        } else {
            Err(anyhow::anyhow!(self.error.unwrap_or_else(|| "unknown error".into())))
        }
    }
}

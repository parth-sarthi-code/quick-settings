use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::services::deviced::{default_socket_path, BluetoothDevice, DaemonRequest, DaemonResponse};
use crate::services::AudioOutput;
use crate::services::network::WifiNetwork;

pub struct DeviceClient {
    socket_path: PathBuf,
}

impl DeviceClient {
    #[allow(dead_code)]
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
        }
    }

    async fn send<T: DeserializeOwned>(&self, request: &DaemonRequest) -> Result<T> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| format!("connect to daemon at {:?}", self.socket_path))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let body = serde_json::to_string(request)?;
        writer.write_all(body.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: DaemonResponse<T> = serde_json::from_str(&line)?;
        resp.into_result()
    }

    pub async fn wifi_list(&self) -> Result<Vec<WifiNetwork>> {
        self.send(&DaemonRequest::WifiList).await
    }

    pub async fn wifi_connected(&self) -> Result<Option<WifiNetwork>> {
        self.send(&DaemonRequest::WifiConnected).await
    }

    pub async fn wifi_connect(&self, ssid: &str) -> Result<()> {
        self.send::<serde_json::Value>(&DaemonRequest::WifiConnect {
            ssid: ssid.to_string(),
        })
        .await?;
        Ok(())
    }

    pub async fn wifi_disconnect(&self) -> Result<()> {
        self.send::<serde_json::Value>(&DaemonRequest::WifiDisconnect)
            .await?;
        Ok(())
    }

    pub async fn bluetooth_list(&self) -> Result<Vec<BluetoothDevice>> {
        self.send(&DaemonRequest::BluetoothList).await
    }

    pub async fn bluetooth_connected(&self) -> Result<Option<BluetoothDevice>> {
        self.send(&DaemonRequest::BluetoothConnected).await
    }

    pub async fn bluetooth_connect(&self, mac: &str) -> Result<()> {
        self.send::<serde_json::Value>(&DaemonRequest::BluetoothConnect {
            mac: mac.to_string(),
        })
        .await?;
        Ok(())
    }

    pub async fn bluetooth_disconnect(&self, mac: &str) -> Result<()> {
        self.send::<serde_json::Value>(&DaemonRequest::BluetoothDisconnect {
            mac: mac.to_string(),
        })
        .await?;
        Ok(())
    }

    pub async fn audio_outputs(&self) -> Result<Vec<AudioOutput>> {
        self.send(&DaemonRequest::AudioOutputs).await
    }

    pub async fn audio_set_default(&self, id: &str) -> Result<()> {
        self.send::<serde_json::Value>(&DaemonRequest::AudioSetDefault {
            id: id.to_string(),
        })
        .await?;
        Ok(())
    }
}

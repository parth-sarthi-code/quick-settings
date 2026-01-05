use anyhow::{anyhow, Result};
use std::collections::HashMap;
use zbus::fdo::ObjectManagerProxy;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, Proxy};

use crate::services::deviced::BluetoothDevice;

#[allow(dead_code)]
const BLUEZ_SERVICE: &str = "org.bluez";
#[allow(dead_code)]
const BLUEZ_DEVICE_IFACE: &str = "org.bluez.Device1";

#[derive(Clone)]
#[allow(dead_code)]
pub struct BluetoothService {
    connection: Option<Connection>,
}

impl BluetoothService {
    #[allow(dead_code)]
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    #[allow(dead_code)]
    pub fn new_stub() -> Self {
        Self { connection: None }
    }

    #[allow(dead_code)]
    pub async fn list_devices(&self) -> Result<Vec<BluetoothDevice>> {
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let proxy = ObjectManagerProxy::builder(connection)
            .destination(BLUEZ_SERVICE)?
            .path("/")?
            .build()
            .await?;

        let objects = proxy.get_managed_objects().await?;
        let mut devices = Vec::new();

        for (_path, ifaces) in objects {
            if let Some(props) = ifaces.get(BLUEZ_DEVICE_IFACE) {
                if let Some(device) = Self::parse_device(props) {
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    #[allow(dead_code)]
    pub async fn connected_device(&self) -> Result<Option<BluetoothDevice>> {
        let devices = self.list_devices().await?;
        Ok(devices.into_iter().find(|d| d.connected))
    }

    #[allow(dead_code)]
    pub async fn connect_device(&self, mac: &str) -> Result<()> {
        let device_path = self.find_device_path(mac).await?;
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let device_proxy = Proxy::new(connection, BLUEZ_SERVICE, device_path, BLUEZ_DEVICE_IFACE).await?;
        device_proxy.call_method("Connect", &()).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn disconnect_device(&self, mac: &str) -> Result<()> {
        let device_path = self.find_device_path(mac).await?;
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let device_proxy = Proxy::new(connection, BLUEZ_SERVICE, device_path, BLUEZ_DEVICE_IFACE).await?;
        device_proxy.call_method("Disconnect", &()).await?;
        Ok(())
    }

    // Helper methods
    async fn find_device_path(&self, mac: &str) -> Result<zbus::zvariant::OwnedObjectPath> {
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let proxy = ObjectManagerProxy::builder(connection)
            .destination(BLUEZ_SERVICE)?
            .path("/")?
            .build()
            .await?;

        let objects = proxy.get_managed_objects().await?;

        objects
            .into_iter()
            .find_map(|(path, ifaces)| {
                if let Some(props) = ifaces.get(BLUEZ_DEVICE_IFACE) {
                    if let Some(address) = props.get("Address").and_then(|v| v.downcast_ref::<String>().ok()) {
                        if address == mac {
                            return Some(path);
                        }
                    }
                }
                None
            })
            .ok_or_else(|| anyhow!("Device {mac} not found"))
    }

    fn parse_device(props: &HashMap<String, OwnedValue>) -> Option<BluetoothDevice> {
        let address = props
            .get("Address")
            .and_then(|v| v.downcast_ref::<String>().ok())
            .map(|s| s.to_string())?;

        let name = props
            .get("Name")
            .and_then(|v| v.downcast_ref::<String>().ok())
            .map(|s| s.to_string())
            .or_else(|| props.get("Alias").and_then(|v| v.downcast_ref::<String>().ok()).map(|s| s.to_string()))
            .unwrap_or_else(|| address.clone());

        let connected = props
            .get("Connected")
            .and_then(|v| v.downcast_ref::<bool>().ok())
            .unwrap_or(false);

        Some(BluetoothDevice {
            name,
            mac: address,
            connected,
        })
    }
}

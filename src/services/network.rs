use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use zbus::{zvariant::Value, Connection, Proxy};

#[allow(dead_code)]
const NM_DEVICE_TYPE_WIFI: u32 = 2;
#[allow(dead_code)]
const AP_SECURITY_NONE: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub strength: u8,
    pub secure: bool,
    pub connected: bool,
    pub path: String,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkService {
    connection: Option<Connection>,
}

impl NetworkService {
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
    pub async fn get_wifi_networks(&self) -> Result<Vec<WifiNetwork>> {
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let mut networks = Vec::new();

        // Find WiFi device and get access points
        if let Some((device_path, wifi_proxy, _active_ssid)) = self.find_wifi_device(connection).await? {
            let active_ssid = self.get_active_ssid(connection, &device_path).await;

            // Request scan and get APs
            let _ = wifi_proxy.call_method("RequestScan", &(HashMap::<String, Value>::new())).await;

            let aps: Vec<zbus::zvariant::OwnedObjectPath> =
                wifi_proxy.call_method("GetAccessPoints", &()).await?.body().deserialize()?;

            let mut ssid_map: HashMap<String, WifiNetwork> = HashMap::new();

            for ap_path in aps {
                if let Ok(network) = Self::parse_access_point(connection, &ap_path, &active_ssid).await {
                    if !network.ssid.is_empty() {
                        ssid_map
                            .entry(network.ssid.clone())
                            .and_modify(|e| {
                                if network.strength > e.strength {
                                    *e = network.clone();
                                }
                            })
                            .or_insert(network);
                    }
                }
            }

            networks.extend(ssid_map.into_values());
        }

        networks.sort_by(|a, b| b.strength.cmp(&a.strength));
        Ok(networks)
    }

    #[allow(dead_code)]
    pub async fn get_connected_network(&self) -> Result<Option<WifiNetwork>> {
        let networks = self.get_wifi_networks().await?;
        Ok(networks.into_iter().find(|n| n.connected))
    }

    #[allow(dead_code)]
    pub async fn connect_network(&self, ssid: &str) -> Result<()> {
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let (device_path, wifi_proxy, _) = self
            .find_wifi_device(connection)
            .await?
            .ok_or_else(|| anyhow!("No WiFi device found"))?;

        let aps: Vec<zbus::zvariant::OwnedObjectPath> =
            wifi_proxy.call_method("GetAccessPoints", &()).await?.body().deserialize()?;

        for ap_path in aps {
            if let Ok(ap_ssid) = Self::get_ap_ssid(connection, &ap_path).await {
                if ap_ssid == ssid {
                    let connection_path = self.find_or_create_connection(connection, ssid).await?;

                    let nm_proxy = Proxy::new(
                        connection,
                        "org.freedesktop.NetworkManager",
                        "/org/freedesktop/NetworkManager",
                        "org.freedesktop.NetworkManager",
                    )
                    .await?;

                    let _: zbus::zvariant::OwnedObjectPath = nm_proxy
                        .call_method("ActivateConnection", &(connection_path, &device_path, &ap_path))
                        .await?
                        .body()
                        .deserialize()?;

                    return Ok(());
                }
            }
        }

        Err(anyhow!("Network {ssid} not found"))
    }

    #[allow(dead_code)]
    pub async fn disconnect_network(&self) -> Result<()> {
        let connection = self.connection.as_ref().ok_or_else(|| anyhow!("No D-Bus connection"))?;

        let nm_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await?;

        let devices: Vec<zbus::zvariant::OwnedObjectPath> =
            nm_proxy.call_method("GetDevices", &()).await?.body().deserialize()?;

        for device_path in devices {
            if let Ok(device_type) = Self::get_device_type(connection, &device_path).await {
                if device_type == NM_DEVICE_TYPE_WIFI {
                    let device_iface = Proxy::new(
                        connection,
                        "org.freedesktop.NetworkManager",
                        device_path,
                        "org.freedesktop.NetworkManager.Device",
                    )
                    .await?;

                    let _: () = device_iface.call_method("Disconnect", &()).await?.body().deserialize()?;
                    return Ok(());
                }
            }
        }

        Err(anyhow!("No WiFi device found to disconnect"))
    }

    // Helper methods
    async fn find_wifi_device(
        &self,
        connection: &Connection,
    ) -> Result<Option<(zbus::zvariant::OwnedObjectPath, Proxy<'_>, Option<String>)>> {
        let nm_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            "org.freedesktop.NetworkManager",
        )
        .await?;

        let devices: Vec<zbus::zvariant::OwnedObjectPath> =
            nm_proxy.call_method("GetDevices", &()).await?.body().deserialize()?;

        for device_path in devices {
            if let Ok(device_type) = Self::get_device_type(connection, &device_path).await {
                if device_type == NM_DEVICE_TYPE_WIFI {
                    let wifi_proxy = Proxy::new(
                        connection,
                        "org.freedesktop.NetworkManager",
                        device_path.clone(),
                        "org.freedesktop.NetworkManager.Device.Wireless",
                    )
                    .await?;

                    let active_ssid = self.get_active_ssid(connection, &device_path).await;
                    return Ok(Some((device_path, wifi_proxy, active_ssid)));
                }
            }
        }

        Ok(None)
    }

    async fn get_device_type(connection: &Connection, device_path: &zbus::zvariant::OwnedObjectPath) -> Result<u32> {
        let proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            device_path,
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let result: u32 = proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.Device", "DeviceType"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        Ok(result)
    }

    async fn get_active_ssid(&self, connection: &Connection, device_path: &zbus::zvariant::OwnedObjectPath) -> Option<String> {
        let device_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            device_path,
            "org.freedesktop.DBus.Properties",
        )
        .await
        .ok()?;

        let active_conn_path: Option<zbus::zvariant::OwnedObjectPath> = device_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.Device", "ActiveConnection"))
            .await
            .ok()?
            .body()
            .deserialize::<Value>()
            .ok()?
            .try_into()
            .ok();

        if let Some(conn_path) = active_conn_path {
            if conn_path.as_str() != "/" {
                let conn_proxy = Proxy::new(
                    connection,
                    "org.freedesktop.NetworkManager",
                    conn_path,
                    "org.freedesktop.DBus.Properties",
                )
                .await
                .ok()?;

                if let Ok(ap_path) = conn_proxy
                    .call_method("Get", &("org.freedesktop.NetworkManager.Connection.Active", "SpecificObject"))
                    .await
                {
                    if let Some(ap_path_val) = ap_path.body().deserialize::<Value>().ok().and_then(|v| v.try_into().ok()) {
                        let ap_path_val: zbus::zvariant::OwnedObjectPath = ap_path_val;
                        if ap_path_val.as_str() != "/" {
                            return Self::get_ap_ssid(connection, &ap_path_val).await.ok();
                        }
                    }
                }
            }
        }

        None
    }

    async fn get_ap_ssid(connection: &Connection, ap_path: &zbus::zvariant::OwnedObjectPath) -> Result<String> {
        let ap_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            ap_path,
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let ssid_bytes: Vec<u8> = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "Ssid"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        Ok(String::from_utf8_lossy(&ssid_bytes).to_string())
    }

    async fn parse_access_point(
        connection: &Connection,
        ap_path: &zbus::zvariant::OwnedObjectPath,
        active_ssid: &Option<String>,
    ) -> Result<WifiNetwork> {
        let ap_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            ap_path,
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let ssid_bytes: Vec<u8> = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "Ssid"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();

        let strength: u8 = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "Strength"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        let flags: u32 = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "Flags"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        let wpa_flags: u32 = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "WpaFlags"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        let rsn_flags: u32 = ap_proxy
            .call_method("Get", &("org.freedesktop.NetworkManager.AccessPoint", "RsnFlags"))
            .await?
            .body()
            .deserialize::<Value>()?
            .try_into()?;

        let secure = flags & 0x1 != 0 || wpa_flags != 0 || rsn_flags != 0;
        let connected = active_ssid.as_ref() == Some(&ssid);

        Ok(WifiNetwork {
            ssid,
            strength,
            secure,
            connected,
            path: ap_path.to_string(),
        })
    }

    async fn find_or_create_connection(
        &self,
        connection: &Connection,
        ssid: &str,
    ) -> Result<zbus::zvariant::OwnedObjectPath> {
        let settings_proxy = Proxy::new(
            connection,
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager/Settings",
            "org.freedesktop.NetworkManager.Settings",
        )
        .await?;

        let conns: Vec<zbus::zvariant::OwnedObjectPath> =
            settings_proxy.call_method("ListConnections", &()).await?.body().deserialize()?;

        for conn_path in &conns {
            let conn_proxy = Proxy::new(
                connection,
                "org.freedesktop.NetworkManager",
                conn_path.clone(),
                "org.freedesktop.NetworkManager.Settings.Connection",
            )
            .await?;

            if let Ok(settings) = conn_proxy.call_method("GetSettings", &()).await {
                if let Ok(map) = settings.body().deserialize::<HashMap<String, HashMap<String, Value>>>() {
                    if let Some(id_val) = map.get("connection").and_then(|cs| cs.get("id")) {
                        if let Ok(id_str) = Self::value_to_string(id_val) {
                            if id_str == ssid {
                                return Ok(conn_path.clone());
                            }
                        }
                    }
                }
            }
        }

        // Create new connection
        let mut connection_settings: HashMap<String, HashMap<String, Value>> = HashMap::new();

        let mut connection_section = HashMap::new();
        connection_section.insert("id".to_string(), Value::new(ssid.to_string()));
        connection_section.insert("type".to_string(), Value::new("802-11-wireless"));
        connection_section.insert("uuid".to_string(), Value::new(Uuid::new_v4().to_string()));
        connection_settings.insert("connection".to_string(), connection_section);

        let mut wifi_section = HashMap::new();
        wifi_section.insert("ssid".to_string(), Value::new(ssid.as_bytes()));
        connection_settings.insert("802-11-wireless".to_string(), wifi_section);

        let new_path: zbus::zvariant::OwnedObjectPath =
            settings_proxy.call_method("AddConnection", &(connection_settings)).await?.body().deserialize()?;

        Ok(new_path)
    }

    fn value_to_string(val: &Value) -> Result<String> {
        Ok(val.try_into()?)
    }
}

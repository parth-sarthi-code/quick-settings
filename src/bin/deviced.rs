use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;

use niri_bar::services::audio::AudioService;
use niri_bar::services::bluetooth::BluetoothService;
use niri_bar::services::deviced::{default_socket_path, DaemonRequest, DaemonResponse};
use niri_bar::services::network::NetworkService;

#[tokio::main]
async fn main() -> Result<()> {
    let socket_path = default_socket_path();
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
    println!("deviced listening on {:?}", socket_path);

    let network = NetworkService::new().await.unwrap_or_else(|e| {
        eprintln!("NetworkService init failed: {} (stub mode)", e);
        NetworkService::new_stub()
    });

    let bluetooth = BluetoothService::new().await.unwrap_or_else(|e| {
        eprintln!("BluetoothService init failed: {} (stub mode)", e);
        BluetoothService::new_stub()
    });

    let audio = AudioService::new();

    tokio::select! {
        res = accept_loop(listener, network, bluetooth, audio) => res?,
        _ = signal::ctrl_c() => {
            println!("deviced shutting down");
        }
    }

    Ok(())
}

async fn accept_loop(
    listener: UnixListener,
    network: NetworkService,
    bluetooth: BluetoothService,
    audio: AudioService,
) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let network = network.clone();
        let bluetooth = bluetooth.clone();
        let audio = audio.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, network, bluetooth, audio).await {
                eprintln!("deviced connection error: {}", e);
            }
        });
    }
}

async fn handle_conn(
    stream: tokio::net::UnixStream,
    network: NetworkService,
    bluetooth: BluetoothService,
    audio: AudioService,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::with_capacity(256); // Pre-allocate
    reader.read_line(&mut line).await?;
    let req: DaemonRequest = serde_json::from_str(&line)?;

    let resp: DaemonResponse<Value> = match req {
        DaemonRequest::Ping => DaemonResponse::ok(json!({"pong": true})),
        DaemonRequest::WifiList => match network.get_wifi_networks().await {
            Ok(list) => DaemonResponse::ok(serde_json::to_value(&list)?),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::WifiConnected => match network.get_connected_network().await {
            Ok(conn) => DaemonResponse::ok(serde_json::to_value(&conn)?),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::WifiConnect { ssid } => match network.connect_network(&ssid).await {
            Ok(_) => DaemonResponse::ok(json!({"connected": true})),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::WifiDisconnect => match network.disconnect_network().await {
            Ok(_) => DaemonResponse::ok(json!({"disconnected": true})),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::BluetoothList => match bluetooth.list_devices().await {
            Ok(list) => DaemonResponse::ok(serde_json::to_value(&list)?),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::BluetoothConnected => match bluetooth.connected_device().await {
            Ok(conn) => DaemonResponse::ok(serde_json::to_value(&conn)?),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::BluetoothConnect { mac } => match bluetooth.connect_device(&mac).await {
            Ok(_) => DaemonResponse::ok(json!({"connected": true})),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::BluetoothDisconnect { mac } => match bluetooth.disconnect_device(&mac).await {
            Ok(_) => DaemonResponse::ok(json!({"disconnected": true})),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::AudioOutputs => match audio.outputs().await {
            Ok(list) => DaemonResponse::ok(serde_json::to_value(&list)?),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
        DaemonRequest::AudioSetDefault { id } => match audio.set_default_output(&id).await {
            Ok(_) => DaemonResponse::ok(json!({"set": true})),
            Err(e) => DaemonResponse::err(e.to_string()),
        },
    };

    let body = serde_json::to_string(&resp)?;
    writer.write_all(body.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

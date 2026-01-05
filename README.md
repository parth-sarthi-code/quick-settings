
# Quick Settings (qs)

A modern, lightweight, high-performance quick settings panel and top bar for Wayland compositors (tested with [niri](https://github.com/YaLTeR/niri)).

## Screenshot

![Panel](screenshots/panel.png)


## Services and Integration

| Component         | Method   | Backend/Tool                                    |
|-------------------|----------|------------------------------------------------|
| WiFi              | D-Bus    | NetworkManager (org.freedesktop.NetworkManager) |
| Bluetooth         | D-Bus    | BlueZ (org.bluez)                               |
| Audio/Volume      | CLI      | wpctl (WirePlumber)                             |
| Brightness        | CLI      | brightnessctl                                   |
| Battery           | File+CLI | /sys/class/power_supply + upower                |
| Power Profile     | CLI      | powerprofilesctl                                |
| Power Actions     | CLI      | systemctl, loginctl                             |
| Warp VPN          | CLI      | warp-cli                                        |
| Top Bar           | IPC      | niri (via $NIRI_SOCKET + socat)                 |

## Manual Installation

1. **Install dependencies:**
    - Rust toolchain
    - GTK4, gtk4-layer-shell, WirePlumber, NetworkManager, BlueZ, brightnessctl, upower, powerprofilesctl, systemctl, loginctl, warp-cli, socat

2. **Clone the repository:**
    ```sh
    git clone <repo-url>
    cd "Quick Settings"
    ```

3. **Build the project:**
    ```sh
    cargo build --release
    ```

4. **Update your niri config** (replace `<path-to-project>` with your actual path):
    ```kdl
    spawn-at-startup "<path-to-project>/target/release/deviced"
    spawn-at-startup "<path-to-project>/target/release/qs"
    ```

5. **(Optional) Update Waybar config for quicksettings:**
    ```json
    "on-click": "pkill -SIGUSR1 qs"
    ```
    - Methods: Shutdown, Reboot, Suspend, Logout
- **Battery:**
    - Service: `org.freedesktop.UPower`
    - Methods: Battery status, percentage

### deviced Daemon (WiFi & Bluetooth)
- **Network:**
    - Service: `org.freedesktop.NetworkManager`
    - Methods: Enable/disable Wi-Fi, show status
- **Bluetooth:**
    - Service: `org.bluez`
    - Methods: Enable/disable, show status

### CLI Tools
- **Audio:**
    - `wpctl set-volume`, `wpctl get-volume`
- **Brightness:**
    - `brightnessctl set`, `brightnessctl get`
- **Power profiles:**
    - `powerprofilesctl set`, `powerprofilesctl get`
- **Systemd:**
    - `systemctl poweroff`, `systemctl reboot`
- **Session:**
    - `loginctl lock-session`, `loginctl terminate-session`
- **VPN:**
    - `warp-cli status`, `warp-cli connect`, `warp-cli disconnect`
- **Media:**
    - `playerctl play-pause`, `playerctl next`, `playerctl previous`

src/
src/

## Project Structure

```
src/
├── main.rs              # Application entry point
├── ipc/                 # IPC event handling
├── state/               # Shared application state
├── services/            # D-Bus, CLI, and system services
├── ui/                  # GTK4 UI components
└── utils/               # Utilities
```

## License

MIT

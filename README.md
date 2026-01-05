# WayLand QS_Panel

A modern quick settings panel and top bar for Wayland compositors.

## Screenshots

![Panel](screenshots/panel.png)

A lightweight, high-performance quick settings panel and top bar for Wayland compositors (tested with [niri](https://github.com/YaLTeR/niri)).

A lightweight, high-performance quick settings panel and top bar for Wayland compositors (tested with [niri](https://github.com/YaLTeR/niri)).

## Services and Integration

| Component         | Method | Backend/Tool                                    |
|-------------------|--------|------------------------------------------------|
| WiFi              | D-Bus  | NetworkManager (org.freedesktop.NetworkManager) |
| Bluetooth         | D-Bus  | BlueZ (org.bluez)                               |
| Audio/Volume      | CLI    | wpctl (WirePlumber)                             |
| Brightness        | CLI    | brightnessctl                                   |
| Battery           | File+CLI | /sys/class/power_supply + upower              |
| Power Profile     | CLI    | powerprofilesctl                                |
| Power Actions     | CLI    | systemctl, loginctl                             |
| Warp VPN          | CLI    | warp-cli                                        |
| Waybar (Top Bar)  | IPC    | niri (via $NIRI_SOCKET + socat)                 |

## Manual Installation

1. Install dependencies:
    - Rust toolchain
    - GTK4, gtk4-layer-shell, WirePlumber, NetworkManager, BlueZ, brightnessctl, upower, powerprofilesctl, systemctl, loginctl, warp-cli, socat

2. Clone the repository:
    ```sh
    git clone <repo-url>
    cd "Quick Settings"
    ```

3. Build the project:
    ```sh
    cargo build --release
    ```

4. Update your niri config (replace <path-to-project> with your actual path):
    ```kdl
    spawn-at-startup "<path-to-project>/target/release/deviced"
    spawn-at-startup "<path-to-project>/target/release/qs"
    ```

5. (Optional) Update Waybar config for quicksettings:
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

See the table above for how each component is integrated and which backend/tool is used.
### Direct File Reads
- **Battery:**
    - `/sys/class/power_supply/BAT0/capacity` (percentage)

## Project Structure

```
src/
├── main.rs
├── ipc/           # niri IPC event handling
├── state/         # Shared state
├── services/      # D-Bus, CLI, and system services
├── ui/            # GTK4 UI components
└── utils/
```

## Building

```bash
cargo build --release
```

## Running

```bash
./target/release/niri-bar
```

Or add to your niri config:

```kdl
spawn-at-startup "niri-bar"
```

## Project Structure

```
src/
├── main.rs              # Application entry point
├── ipc/
│   └── niri.rs         # niri IPC event handling
├── state/
│   └── mod.rs          # Shared application state
├── ui/
│   ├── bar.rs          # Main GTK4 top bar window
│   └── widgets.rs      # Widget utilities
└── utils/
    ├── mod.rs
    └── time.rs         # Time formatting utilities
```

## Performance Characteristics

- **Low Latency**: IPC updates are instant via async event stream
- **Minimal Polling**: Only clock updates every 60 seconds
- **Non-blocking**: UI updates marshalled to GTK thread via glib
- **Optimized Build**: LTO enabled, single codegen unit

## Current Behavior

- Bar height: 32px
- Background: Semi-transparent dark (rgba 30,30,30,0.95)
- Workspace indicator: Highlighted with blue background
- Window title: Truncated at 60 characters with ellipsis
- Clock format: `HH:MM  Day Mon DD` (e.g., "23:26  Fri Jan 03")

## Future Enhancements

Phase 2 could include:
- System tray
- Quick settings
- Notifications
- Volume/brightness controls
- Network indicator
- Battery status

## License

MIT

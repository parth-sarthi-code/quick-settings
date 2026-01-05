# niri-bar

A lightweight, high-performance top bar for the [niri](https://github.com/YaLTeR/niri) Wayland compositor.

## Features

✅ **Phase 1 (MVP) - Complete**
- Workspace indicator showing current workspace (left)
- Active window title display (left, next to workspace)
- Clock with time and date (center)
- Real-time updates via niri IPC
- Layer-shell integration for proper panel behavior

## Architecture

- **Single Process**: One Rust binary with internal separation of concerns
- **Core State Manager**: Async IPC listener + shared state
- **GTK4 UI Layer**: Rendering only, never blocks IPC
- **Data Flow**: `niri IPC → tokio listener → RwLock state → GTK UI`

## Requirements

- Rust 1.70+
- GTK4
- gtk4-layer-shell
- niri compositor

## Building

```bash
cargo build --release
```

## Running

```bash
./target/release/niri-bar
```

Or add to your niri config to start automatically:

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

# niri-bar Phase 2: Quick Settings

This document outlines Phase 2 implementation of the niri-bar project, which adds a GNOME-like Quick Settings panel to the existing GTK4 top bar.

## Architecture Overview

```
niri-bar (Phase 2)
├── Top Bar (Phase 1)
│   └── Click ⚙ → Open Quick Settings
└── Quick Settings Panel (NEW)
    ├── Audio Controls → PipeWire/WirePlumber D-Bus
    ├── Media Controls → MPRIS (org.mpris.MediaPlayer2)
    └── Power Actions → logind D-Bus
```

## Key Design Decisions

### 1. **Separation of Concerns**
- **UI Layer** (`src/ui/quick_settings.rs`): GTK4 widgets only, no business logic
- **Service Layer** (`src/services/`): D-Bus communication, state management
- **State** (`src/state/mod.rs`): Shared application state (phase 1)

### 2. **Wayland / Layer-Shell Behavior**
The Quick Settings panel:
- Uses `gtk-layer-shell` for Wayland integration
- Anchored to top edge, positioned below top bar
- **No exclusive zone** (unlike top bar)
- **Keyboard mode: NONE** - does not steal focus from normal windows
- Closes when clicking outside (popover behavior)

### 3. **Threading Model**
```
Main Thread (GTK)         Async Thread (D-Bus)
    ↓                            ↓
UI Callbacks          → RwLock<AppState> ← D-Bus Listeners
    ↑                            ↑
State Polling         ← Tokio Task spawning
```

- GTK updates run on main thread via `glib::spawn_future_local`
- D-Bus calls are async, use `zbus` library
- No blocking calls in UI callbacks

## Project Structure

```
src/
├── main.rs                     # App lifecycle, services init
├── state/
│   └── mod.rs                 # AppState, workspace/window tracking
├── services/
│   ├── mod.rs                 # Service types & state definitions
│   ├── audio.rs               # PipeWire/WirePlumber (volume, mute)
│   ├── media.rs               # MPRIS player control
│   └── power.rs               # logind (shutdown, reboot, logout)
├── ui/
│   ├── bar.rs                 # Top bar with Quick Settings button
│   ├── quick_settings.rs      # Quick Settings popover UI (NEW)
│   ├── widgets.rs             # Utility widgets
│   └── mod.rs                 # UI module exports
├── ipc/
│   └── niri.rs                # niri IPC listener (phase 1)
└── utils/
    └── time.rs                # Time formatting
```

## Phase 2 Features Implemented

### ✅ Quick Settings UI Skeleton

**File:** `src/ui/quick_settings.rs`

- Floating popover panel using GTK4
- Positioned below top bar via layer-shell
- Clean layout with icon headers
- Sections: Audio, Media, Power
- CSS-styled buttons and sliders

**Key Features:**
```rust
pub struct QuickSettings {
    window: ApplicationWindow,
    popover: Popover,
    is_visible: Arc<RwLock<bool>>,
}
```

- `toggle()` - Show/hide panel (async-safe)
- Click outside to close (native popover behavior)
- No focus stealing (KeyboardMode::None)

### ✅ Audio Service (PipeWire Backend)

**File:** `src/services/audio.rs`

```rust
pub struct AudioService {
    connection: Connection,  // zbus D-Bus connection
    current_state: Arc<RwLock<AudioState>>,
}
```

**Planned D-Bus Integration:**
- Target: `org.PulseAudio.Core1` (PipeWire compatibility layer)
- Methods: Get/Set volume, Mute/Unmute

**Current Capabilities:**
- `set_volume(f64)` - Set volume 0.0-1.0
- `toggle_mute()` - Toggle mute state
- `state()` - Get current AudioState
- `is_muted()` - Check mute status

### ✅ Media Service (MPRIS Backend)

**File:** `src/services/media.rs`

```rust
pub struct MediaService {
    connection: Connection,
    current_state: Arc<RwLock<MediaState>>,
}
```

**Planned D-Bus Integration:**
- Standard: `org.mpris.MediaPlayer2` D-Bus interface
- Auto-detects active media player (mpd, Spotify, etc.)

**Current Capabilities:**
- `play_pause()` - Toggle play/pause
- `next()` - Next track
- `previous()` - Previous track
- `state()` - Get current MediaState (title, artist, playing)

### ✅ Power Service (logind Backend)

**File:** `src/services/power.rs`

```rust
pub struct PowerService {
    connection: Connection,
}
```

**Planned D-Bus Integration:**
- Standard: `org.freedesktop.login1.Manager`
- Methods: `PowerOff()`, `Reboot()`, session `Terminate()`

**Current Capabilities:**
- `execute(PowerActionType)` - Execute power action
- Supports: Shutdown, Reboot, Logout

## UI Layout

```
┌──────────────────────────────────┐
│ 🔊 Volume  [============|   ] 75% │
│ 🔇 Mute    [ OFF ]               │
├──────────────────────────────────┤
│ 🎵 Track Title (if available)    │
│ ⏮  ⏯  ⏭  (play controls)        │
├──────────────────────────────────┤
│ Logout    ⟳ Reboot   ⏻ Shutdown │
└──────────────────────────────────┘
```

**CSS Styling:**
- Dark theme: rgba(30-70, 30-70, 30-70)
- Rounded corners (6px borders)
- Hover states with 0.2 opacity boost
- Touch-friendly sizes (8px padding minimum)

## Phase 2 Integration Points

### 1. Top Bar Button Click
**File:** `src/ui/bar.rs` (lines ~75-85)

```rust
// Right section of top bar
let settings_btn = Button::with_label("⚙");
settings_btn.connect_clicked(move |_| {
    let qs = Arc::clone(&quick_settings_clone);
    glib::spawn_future_local(async move {
        qs.toggle().await;
    });
});
```

### 2. Service Initialization (Phase 2+)
When services are needed, initialize in main thread:

```rust
let audio_svc = AudioService::new().await?;
let media_svc = MediaService::new().await?;
let power_svc = PowerService::new().await?;
```

### 3. State Updates (Phase 2+)
UI polls service state every 100ms:

```rust
let audio_state = audio_svc.state().await;
volume_slider.set_value(audio_state.volume * 100.0);
```

## Dependencies

**New in Phase 2:**
- `zbus = { version = "4.4", features = ["tokio"] }` - D-Bus communication
- `futures-util = "0.3"` - Async utilities

**Existing (Phase 1):**
- `gtk4 = "0.9"` - GTK4 bindings
- `gtk4-layer-shell = "0.4"` - Wayland layer-shell
- `tokio = { version = "1.40", features = ["full"] }` - Async runtime
- `serde = { version = "1.0", features = ["derive"] }` - Serialization

## Next Steps (Phase 3)

1. **D-Bus Integration**
   - Implement actual `zbus` calls for PipeWire volume control
   - Listen to D-Bus signals for volume/media changes
   - Auto-detect media player presence

2. **Live State Updates**
   - Real-time listeners instead of polling
   - Debounce frequent signals (e.g., volume slider)
   - Update UI immediately on D-Bus events

3. **Additional Features**
   - Wi-Fi network selection
   - Bluetooth device pairing
   - System notifications panel
   - Quick toggles (Night Light, Do Not Disturb)

4. **Polish & Performance**
   - Smooth animations for panel open/close
   - Keyboard shortcuts (Super+S?)
   - Screen position awareness (right-aligned on ultrawide)
   - Accessibility improvements

## Testing

**Build:**
```bash
cargo build --release
```

**Run:**
```bash
./target/release/niri-bar
```

**Click ⚙ in top bar to open Quick Settings panel**

## Known Limitations

- Audio/Media/Power services have stub implementations (eprintln! only)
- D-Bus calls not yet wired up (next phase)
- No live state updates from D-Bus listeners
- Panel position fixed (not responsive to screen size changes)
- CSS colors use hardcoded hex values

## Architecture Notes

### Why Arc<RwLock<>>?
- `Arc`: Thread-safe reference counting for shared ownership
- `RwLock`: Allow multiple readers (UI polling) + one writer (D-Bus listener)
- Alternative: Would need `Arc<Mutex<>>` or `Arc<tokio::sync::Mutex<>>` if strict mutual exclusion needed

### Why glib::spawn_future_local?
- GTK callbacks must run on main thread
- `glib::spawn_future_local` bridges async code to GTK main loop
- Prevents "thread panicked" errors when accessing GTK state

### Why gtk-layer-shell?
- Native Wayland protocol support
- Proper layer positioning (top, overlay, background)
- Exclusive zone for panels
- Keyboard mode control (essential for non-focus-stealing panels)

## Debugging

**Enable debug output:**
```bash
RUST_LOG=debug ./target/release/niri-bar 2>&1 | grep AUDIO/MEDIA/POWER
```

Current logging tags:
- `[AUDIO] Volume set to X%`
- `[AUDIO] Mute toggled: true/false`
- `[MEDIA] PlayPause: playing/paused`
- `[MEDIA] Next track`
- `[MEDIA] Previous track`
- `[POWER] Shutdown/Reboot/Logout initiated`

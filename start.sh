#!/bin/bash
# Startup script for niri-bar and deviced daemon

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/target/release"

# Check if binaries exist
if [ ! -f "$BIN_DIR/niri-bar" ] || [ ! -f "$BIN_DIR/deviced" ]; then
    echo "Error: Binaries not found. Run 'cargo build --release' first."
    exit 1
fi

# Kill existing instances
pkill -f "deviced" 2>/dev/null
pkill -f "niri-bar" 2>/dev/null
sleep 0.5

# Start daemon in background
echo "Starting deviced daemon..."
"$BIN_DIR/deviced" > /tmp/deviced.log 2>&1 &
DAEMON_PID=$!

# Wait for daemon socket
SOCKET="${XDG_RUNTIME_DIR:-/tmp}/niri-bar-deviced.sock"
for i in {1..10}; do
    if [ -S "$SOCKET" ]; then
        echo "Daemon ready (PID: $DAEMON_PID)"
        break
    fi
    sleep 0.2
done

if [ ! -S "$SOCKET" ]; then
    echo "Error: Daemon failed to start"
    exit 1
fi

# Start bar
echo "Starting niri-bar..."
"$BIN_DIR/niri-bar"

# Cleanup on exit
kill $DAEMON_PID 2>/dev/null

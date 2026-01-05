#!/bin/bash
# Startup script for waybar with quicksettings

BIN_DIR="/home/parth/bar/target/release"

# Kill existing instances
pkill -f "deviced" 2>/dev/null
pkill -f "niri-bar" 2>/dev/null
pkill -f "waybar" 2>/dev/null
sleep 0.5

# Start daemon
echo "Starting deviced..."
"$BIN_DIR/deviced" >/dev/null 2>&1 &
sleep 0.5

# Start quicksettings backend (hidden, triggered by waybar)
echo "Starting quicksettings..."
"$BIN_DIR/niri-bar" >/dev/null 2>&1 &
sleep 0.5

# Start waybar
echo "Starting waybar..."
waybar >/dev/null 2>&1 &

echo "All services started!"

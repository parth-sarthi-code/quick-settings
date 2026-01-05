#!/usr/bin/env bash

# Get active window title from niri using IPC
if [ -z "$NIRI_SOCKET" ]; then
    echo '{"text":"Desktop","tooltip":"No niri socket"}'
    exit 0
fi

# Send FocusedWindow request to niri IPC socket
response=$(echo '"FocusedWindow"' | socat -t 1 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)

if [ -z "$response" ]; then
    echo '{"text":"Desktop","tooltip":"No response from niri"}'
    exit 0
fi

# Parse the JSON response
# Response format: {"Ok":{"FocusedWindow":{...}}} or {"Ok":null}
title=$(echo "$response" | jq -r '.Ok.FocusedWindow.title // "Desktop"')
app_id=$(echo "$response" | jq -r '.Ok.FocusedWindow.app_id // ""')

# If no focused window, show Desktop
if [ "$title" = "null" ] || [ -z "$title" ]; then
    title="Desktop"
fi

# Truncate title if too long
if [ ${#title} -gt 50 ]; then
    title="${title:0:47}..."
fi

# Output in waybar JSON format
if [ -n "$app_id" ] && [ "$app_id" != "null" ]; then
    echo "{\"text\":\"$title\",\"tooltip\":\"$app_id: $title\"}"
else
    echo "{\"text\":\"$title\",\"tooltip\":\"$title\"}"
fi

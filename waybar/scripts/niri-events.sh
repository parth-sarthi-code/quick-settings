#!/usr/bin/env bash

# Niri event stream handler for waybar
# This script listens to niri event stream and outputs updates for both
# active window and workspace in real-time

if [ -z "$NIRI_SOCKET" ]; then
    exit 1
fi

# Function to output current state as JSON for waybar
output_state() {
    local window_title="$1"
    local window_app="$2"
    local ws_idx="$3"
    local ws_name="$4"
    
    # Format window text
    if [ -z "$window_title" ] || [ "$window_title" = "null" ]; then
        window_text="Desktop"
        window_tooltip="No window focused"
    else
        # Truncate if too long
        if [ ${#window_title} -gt 50 ]; then
            window_text="${window_title:0:47}..."
        else
            window_text="$window_title"
        fi
        
        if [ -n "$window_app" ] && [ "$window_app" != "null" ]; then
            window_tooltip="$window_app: $window_title"
        else
            window_tooltip="$window_title"
        fi
    fi
    
    # Format workspace text
    if [ -n "$ws_name" ] && [ "$ws_name" != "null" ]; then
        ws_text="󰍹 $ws_name"
        ws_tooltip="Workspace: $ws_name ($ws_idx)"
    else
        ws_text="󰍹 $((ws_idx + 1))"
        ws_tooltip="Workspace $((ws_idx + 1))"
    fi
    
    echo "WINDOW:{\"text\":\"$window_text\",\"tooltip\":\"$window_tooltip\"}"
    echo "WORKSPACE:{\"text\":\"$ws_text\",\"tooltip\":\"$ws_tooltip\"}"
}

# Get initial state
response=$(echo '"Workspaces"' | socat -t 1 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)
if [ -n "$response" ]; then
    workspaces=$(echo "$response" | jq -r '.Ok.Workspaces // []')
    focused_ws=$(echo "$workspaces" | jq -r '.[] | select(.is_focused == true)')
    ws_idx=$(echo "$focused_ws" | jq -r '.idx // 0')
    ws_name=$(echo "$focused_ws" | jq -r '.name // ""')
else
    ws_idx=0
    ws_name=""
fi

response=$(echo '"FocusedWindow"' | socat -t 1 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)
if [ -n "$response" ]; then
    window_title=$(echo "$response" | jq -r '.Ok.FocusedWindow.title // ""')
    window_app=$(echo "$response" | jq -r '.Ok.FocusedWindow.app_id // ""')
else
    window_title=""
    window_app=""
fi

# Output initial state
output_state "$window_title" "$window_app" "$ws_idx" "$ws_name"

# Listen to event stream for updates
echo '"EventStream"' | socat - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null | while IFS= read -r event; do
    event_type=$(echo "$event" | jq -r '.Ok | keys[0]')
    
    case "$event_type" in
        WindowFocusChanged)
            # Window focus changed - get new focused window
            window_id=$(echo "$event" | jq -r '.Ok.WindowFocusChanged.id')
            if [ "$window_id" = "null" ] || [ -z "$window_id" ]; then
                window_title=""
                window_app=""
            else
                # Need to query for window details
                response=$(echo '"FocusedWindow"' | socat -t 0.5 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)
                window_title=$(echo "$response" | jq -r '.Ok.FocusedWindow.title // ""')
                window_app=$(echo "$response" | jq -r '.Ok.FocusedWindow.app_id // ""')
            fi
            output_state "$window_title" "$window_app" "$ws_idx" "$ws_name"
            ;;
            
        WindowOpenedOrChanged)
            # Window changed - check if it's focused
            is_focused=$(echo "$event" | jq -r '.Ok.WindowOpenedOrChanged.window.is_focused')
            if [ "$is_focused" = "true" ]; then
                window_title=$(echo "$event" | jq -r '.Ok.WindowOpenedOrChanged.window.title // ""')
                window_app=$(echo "$event" | jq -r '.Ok.WindowOpenedOrChanged.window.app_id // ""')
                output_state "$window_title" "$window_app" "$ws_idx" "$ws_name"
            fi
            ;;
            
        WorkspaceActivated)
            # Workspace changed
            workspace_id=$(echo "$event" | jq -r '.Ok.WorkspaceActivated.id')
            focused=$(echo "$event" | jq -r '.Ok.WorkspaceActivated.focused')
            if [ "$focused" = "true" ]; then
                # Query workspace details
                response=$(echo '"Workspaces"' | socat -t 0.5 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)
                workspaces=$(echo "$response" | jq -r '.Ok.Workspaces // []')
                focused_ws=$(echo "$workspaces" | jq -r '.[] | select(.is_focused == true)')
                ws_idx=$(echo "$focused_ws" | jq -r '.idx // 0')
                ws_name=$(echo "$focused_ws" | jq -r '.name // ""')
                output_state "$window_title" "$window_app" "$ws_idx" "$ws_name"
            fi
            ;;
    esac
done

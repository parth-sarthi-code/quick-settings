#!/usr/bin/env bash

# Workspace pill with dots; active segment elongated and brighter using Pango markup

if [ -z "$NIRI_SOCKET" ]; then
    jq -nc --arg text "?" --arg tooltip "No niri socket" '{"text":$text,"tooltip":$tooltip}'
    exit 0
fi

response=$(echo '"Workspaces"' | socat -t 1 - UNIX-CONNECT:"$NIRI_SOCKET" 2>/dev/null)

if [ -z "$response" ]; then
    jq -nc --arg text "?" --arg tooltip "No response from niri" '{"text":$text,"tooltip":$tooltip}'
    exit 0
fi

workspaces_json=$(echo "$response" | jq -c '.Ok.Workspaces // []')

# If nothing comes back, show a single placeholder dot
if [ "$workspaces_json" = "[]" ]; then
    jq -nc --arg text "•" --arg tooltip "Workspace 1" '{"text":$text,"tooltip":$tooltip}'
    exit 0
fi

# Sort workspaces by idx so dots render in the right order
mapfile -t ws_list < <(echo "$workspaces_json" | jq -r 'sort_by(.idx)[] | "\(.idx // 0) \(.is_focused // false)"')

segments=()
focused_idx=0

for ws in "${ws_list[@]}"; do
    idx=${ws%% *}
    is_focused=${ws#* }

    if [ "$is_focused" = "true" ]; then
        focused_idx=$((idx + 1))
        # Active: single dot padded to read as a pill, greyish black
        segments+=("<span foreground='#1a1a1a' font_desc='14' letter_spacing='400'>&#8239;●&#8239;</span>")
    else
        # Inactive: larger dot
        segments+=("<span foreground='#ffffffd9' font_desc='14'>●</span>")
    fi
done

# Join segments with narrow no-break space for tighter grouping
joined="${segments[0]}"
if [ ${#segments[@]} -gt 1 ]; then
    for seg in "${segments[@]:1}"; do
        joined+="&#8239;${seg}"
    done
fi

tooltip="Workspace $focused_idx"

jq -nc --arg text "$joined" --arg tooltip "$tooltip" '{"text":$text,"tooltip":$tooltip}'

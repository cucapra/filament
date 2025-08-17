#!/bin/bash

set -eu -o pipefail

# Check if output directory argument is provided
if [ $# -ne 1 ]; then
    echo "Usage: $0 <output_directory>" >&2
    echo "Example: $0 filament-results" >&2
    exit 1
fi

# Extract and validate output directory
output_dir="$1"
monitor_dir="$output_dir/_monitor"

# Check if output directory exists
if [ ! -d "$output_dir" ]; then
    echo "Error: Output directory does not exist: $output_dir" >&2
    exit 1
fi

# Check if monitor directory exists
if [ ! -d "$monitor_dir" ]; then
    echo "Error: Monitor directory not found: $monitor_dir" >&2
    echo "Expected structure: $output_dir/_monitor/*.runlog" >&2
    exit 1
fi

# Check if running inside tmux
if [ -z "${TMUX:-}" ]; then
    echo "Error: Not running inside tmux session" >&2
    echo "Start tmux first: tmux new-session" >&2
    exit 1
fi

# Find all runlog files
runlogs=()
while IFS= read -r -d '' file; do
    runlogs+=("$file")
done < <(find "$monitor_dir" -name "*.runlog" -print0 2>/dev/null || true)

if [ ${#runlogs[@]} -eq 0 ]; then
    echo "No .runlog files found in $monitor_dir" >&2
    echo "Synthesis jobs may not have started yet or no logs created" >&2
    exit 1
fi

echo "Found ${#runlogs[@]} log files to monitor"

# Create detached window with directory name to avoid stealing focus
window_name=$(basename "$output_dir")
tmux new-window -d -n "$window_name"

# Setup monitoring by passing commands directly to split-window
num_logs=${#runlogs[@]}

# Start with first log in the initial pane (window already exists)
tmux send-keys -t "$window_name" "tail -f '${runlogs[0]}'" C-m

# Create additional panes with commands passed directly to split-window
for ((i=1; i<num_logs; i++)); do
    # Split and run tail command directly in the new pane
    tmux split-window -t "$window_name" -d "tail -f '${runlogs[$i]}'"
done

# Apply tiled layout for optimal viewing of all panes
tmux select-layout -t "$window_name" tiled

# Get current window number for user reference
current_window=$(tmux display-message -p '#I')

echo "Monitoring window '$window_name' created with ${#runlogs[@]} panes"
echo "Switch to monitoring: Ctrl-b + <window-number> or tmux select-window -t $window_name"
echo "Current window: $current_window"
echo ""
echo "Log files being monitored:"
for log in "${runlogs[@]}"; do
    echo "  - $(basename "$log")"
done
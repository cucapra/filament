#!/bin/bash

set -eu -o pipefail

folders="$@"

# Output file
output_file="res.json"

# Temporary file for JSON array
temp_file=$(mktemp)

# Collect all entries in array
echo "[" > "$temp_file"

# Track if this is the first entry (for comma handling)
first=true

# Find all conv2d synthesis output directories
for dir in $folders; do
    if [ -d "$dir" ]; then
        echo "Processing $dir..." >&2

        # Extract implementation summary using synthrep
        summary=$(synthrep summary -d "$dir" | \
          jq --arg name "$dir" '{ lut: .impl.summary.lut, registers: .impl.summary.registers, meet_timing: .meet_timing, frequency: .frequency, wns: .worst_slack } + {name: $name}')

        if [ -n "$summary" ] && [ "$summary" != "null" ]; then
            # Add comma if not first entry
            if [ "$first" = false ]; then
                echo "," >> "$temp_file"
            else
                first=false
            fi

            # Create JSON object with name, ii, and resource counts
            echo "$summary" >> "$temp_file"
              # jq \
                # --arg name "$dir" \
                # '. + {name: $name} | {name, lut, dsp, brams, registers, carry8, f7_muxes, f8_muxes, f9_muxes}' >> "$temp_file"
        fi
    fi
done

# Close JSON array
echo "]" >> "$temp_file"

# Transform to result.json format
jq '{
    meet_timing: [.[] | {(.name): .meet_timing}] | add,
    frequency: [.[] | {(.name): .frequency}] | add,
    wns: [.[] | {(.name): .wns}] | add,
    luts: [.[] | {(.name): .lut}] | add,
    registers: [.[] | {(.name): .registers}] | add
}' "$temp_file"

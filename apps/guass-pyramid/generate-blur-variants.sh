#!/bin/bash

set -eu -o pipefail

# Script to generate blur variants with different N values and compile them to SystemVerilog

if [ $# -ne 1 ]; then
    echo "Usage: $0 <output_directory>"
    echo "Generates blur variants for N=1,2,4,8,16 and compiles them to .sv files"
    exit 1
fi

OUTPUT_DIR="$1"

# Check if output directory exists, create if not
if [ ! -d "$OUTPUT_DIR" ]; then
    mkdir -p "$OUTPUT_DIR"
fi

# Array of N values to generate
N_VALUES=(1 2 4 8 16)

# Temporary files array for cleanup
TEMP_FILES=()

# Cleanup function
cleanup() {
    echo "Cleaning up temporary files..."
    for temp_file in "${TEMP_FILES[@]}"; do
        if [ -f "$temp_file" ]; then
            rm "$temp_file"
        fi
    done
}

# Set trap for cleanup on exit
trap cleanup EXIT

echo "Generating blur variants for N values: ${N_VALUES[*]}"

for N in "${N_VALUES[@]}"; do
    echo "Processing N=$N..."

    # Generate temporary file name
    TEMP_FILE="blur_${N}_temp.fil"
    TEMP_FILES+=("$TEMP_FILE")

    # Replace {N} with specific value in blur.fil
    sed "s/{N}/$N/g" blur.fil > "$TEMP_FILE"

    # Compile the file and save to .sv file
    OUTPUT_FILE="$OUTPUT_DIR/blur_$N.sv"
    INTERFACE_FILE="$OUTPUT_DIR/blur_$N.json"
    echo "  Compiling to $OUTPUT_FILE..."

    if cargo run -q -- -l ../../ "$TEMP_FILE" > "$OUTPUT_FILE"; then
        echo "  ✓ Successfully compiled blur_$N.sv"
    else
        echo "  ✗ Failed to compile blur_$N.sv"
        echo "  Check $OUTPUT_FILE for error details"
    fi

    if cargo run -q -- --dump-interface -l ../../ "$TEMP_FILE" > "$INTERFACE_FILE"; then
        echo "  ✓ Successfully generated interface for blur_$N.sv"
    else
        echo "  ✗ Failed to generate interface for blur_$N.sv"
        echo "  Check $OUTPUT_FILE for error details"
    fi
done

echo "Generation complete. Output files saved to: $OUTPUT_DIR"
echo "Generated files:"
for N in "${N_VALUES[@]}"; do
    echo "  - blur_$N.sv"
done

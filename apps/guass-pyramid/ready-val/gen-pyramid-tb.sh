#!/bin/bash
set -euo pipefail

# Script to generate pyramid testbench with custom parameters
# Usage: ./gen-pyramid-tb.sh <blur0_n> <blur1_n> <blur_up_n>

usage() {
    echo "Usage: $0 <blur0_n> <blur1_n> <blur_up_n>"
    echo ""
    echo "Generate pyramid testbench SystemVerilog file with custom parameters."
    echo ""
    echo "Arguments:"
    echo "  blur0_n   - N parameter for Blur0 module (positive integer)"
    echo "  blur1_n   - N parameter for Blur1 module (positive integer)"
    echo "  blur_up_n - N parameter for BlurUp module (positive integer)"
    echo ""
    echo "Examples:"
    echo "  $0 4 4 4        # Fast simulation with small chunks"
    echo "  $0 16 16 16     # Default parameters"
    echo "  $0 1 8 16       # Mixed parameters for testing"
    echo ""
    echo "Output:"
    echo "  Prints the generated SystemVerilog testbench to stdout"
    exit 1
}

validate_param() {
    local param_name="$1"
    local param_value="$2"

    if ! [[ "$param_value" =~ ^[0-9]+$ ]] || [ "$param_value" -eq 0 ]; then
        echo "Error: $param_name must be a positive integer, got: '$param_value'" >&2
        exit 1
    fi
}

# Check if we have exactly 3 arguments
if [ $# -ne 3 ]; then
    echo "Error: Expected exactly 3 arguments, got $#" >&2
    echo "" >&2
    usage
fi

# Extract and validate parameters
blur0_n="$1"
blur1_n="$2"
blur_up_n="$3"

validate_param "blur0_n" "$blur0_n"
validate_param "blur1_n" "$blur1_n"
validate_param "blur_up_n" "$blur_up_n"

# Check if source file exists
source_file="tests/pyramid_tb.sv"
if [ ! -f "$source_file" ]; then
    echo "Error: Source file '$source_file' not found" >&2
    echo "Make sure you're running this script from the correct directory" >&2
    exit 1
fi

# Generate the testbench by substituting parameters
sed "s/parameter Blur0_N = 16;/parameter Blur0_N = $blur0_n;/" "$source_file" | \
sed "s/parameter Blur1_N = 16;/parameter Blur1_N = $blur1_n;/" | \
sed "s/parameter BlurUp_N = 16;/parameter BlurUp_N = $blur_up_n;/"

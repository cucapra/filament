#!/bin/bash
set -euo pipefail

# Script to generate pyramid testbench or implementation with custom parameters
# Usage: ./gen-pyramid-tb.sh <Blur0_N> <Blur1_N> <BlurUp_N> <output_type>

usage() {
    echo "Usage: $0 <Blur0_N> <Blur1_N> <BlurUp_N> <output_type>"
    echo ""
    echo "Generate pyramid testbench or implementation SystemVerilog file with custom parameters."
    echo ""
    echo "Arguments:"
    echo "  Blur0_N     - N parameter for Blur0 module (must be 1, 2, 4, 8, or 16)"
    echo "  Blur1_N     - N parameter for Blur1 module (must be 1, 2, 4, 8, or 16)"
    echo "  BlurUp_N    - N parameter for BlurUp module (must be 1, 2, 4, 8, or 16)"
    echo "  output_type - Either 'testbench' or 'implementation'"
    echo ""
    echo "CONSTRAINT: All three N parameters must be equal (due to AetherlingConv limitation)"
    echo ""
    echo "Output Types:"
    echo "  testbench     - Generate complete testbench with test harness"
    echo "  implementation - Generate standalone implementation (conv + pyramid modules)"
    echo ""
    echo "Examples:"
    echo "  $0 4 4 4 testbench        # Fast simulation testbench with N=4"
    echo "  $0 16 16 16 testbench     # Default testbench with N=16"
    echo "  $0 8 8 8 implementation   # Standalone implementation with N=8"
    echo ""
    echo "Output:"
    echo "  Prints the generated SystemVerilog to stdout"
    exit 1
}

validate_n() {
    local n="$1"

    if ! [[ "$n" =~ ^[0-9]+$ ]]; then
        echo "Error: N must be a positive integer, got: '$n'" >&2
        exit 1
    fi

    case "$n" in
        1|2|4|8|16)
            ;;
        *)
            echo "Error: N must be one of: 1, 2, 4, 8, 16, got: '$n'" >&2
            exit 1
            ;;
    esac
}

validate_output_type() {
    local output_type="$1"

    case "$output_type" in
        testbench|implementation)
            ;;
        *)
            echo "Error: output_type must be 'testbench' or 'implementation', got: '$output_type'" >&2
            exit 1
            ;;
    esac
}

# Check if we have exactly 4 arguments
if [ $# -ne 4 ]; then
    echo "Error: Expected exactly 4 arguments, got $#" >&2
    echo "" >&2
    usage
fi

# Extract and validate parameters
blur0_n="$1"
blur1_n="$2"
blurup_n="$3"
output_type="$4"

validate_n "$blur0_n"
validate_n "$blur1_n"
validate_n "$blurup_n"
validate_output_type "$output_type"

# Enforce constraint: all N parameters must be equal
if [ "$blur0_n" != "$blur1_n" ] || [ "$blur1_n" != "$blurup_n" ] || [ "$blur0_n" != "$blurup_n" ]; then
    echo "Error: All three N parameters must be equal due to AetherlingConv limitation" >&2
    echo "Got: Blur0_N=$blur0_n, Blur1_N=$blur1_n, BlurUp_N=$blurup_n" >&2
    echo "Only one AetherlingConv implementation can be used at a time" >&2
    exit 1
fi

# Use common N value
n="$blur0_n"

# Check if required files exist
if [ "$output_type" = "testbench" ]; then
    source_file="tests/pyramid_tb.sv"
    if [ ! -f "$source_file" ]; then
        echo "Error: Source file '$source_file' not found" >&2
        echo "Make sure you're running this script from the correct directory" >&2
        exit 1
    fi
else
    # implementation mode - check for conv file and valid-hold.sv
    conv_file="conv/${n}.sv"
    if [ ! -f "$conv_file" ]; then
        echo "Error: Conv file '$conv_file' not found" >&2
        echo "Available conv files:" >&2
        ls conv/*.sv 2>/dev/null | sed 's/^/  /' >&2 || echo "  No conv files found" >&2
        exit 1
    fi

    if [ ! -f "valid-hold.sv" ]; then
        echo "Error: valid-hold.sv not found" >&2
        echo "Make sure you're running this script from the correct directory" >&2
        exit 1
    fi
fi

# Generate output based on type
if [ "$output_type" = "testbench" ]; then
    # Generate testbench with the specified N parameters
    sed "s/parameter Blur0_N = 16;/parameter Blur0_N = $blur0_n;/" "$source_file" | \
    sed "s/parameter Blur1_N = 16;/parameter Blur1_N = $blur1_n;/" | \
    sed "s/parameter BlurUp_N = 16;/parameter BlurUp_N = $blurup_n;/"
else
    # Generate implementation by concatenating conv file and valid-hold.sv
    echo "// Generated implementation with Blur0_N=$blur0_n, Blur1_N=$blur1_n, BlurUp_N=$blurup_n"
    echo "// All parameters are equal: N=$n"
    echo "// AetherlingConv module from conv/${n}.sv:"
    echo ""
    cat "conv/${n}.sv"
    echo ""
    echo "// Pyramid and supporting modules from valid-hold.sv:"
    echo ""
    sed "s/parameter Blur0_N = 16/parameter Blur0_N = $blur0_n/" "valid-hold.sv" | \
    sed "s/parameter Blur1_N = 16/parameter Blur1_N = $blur1_n/" | \
    sed "s/parameter BlurUp_N = 16/parameter BlurUp_N = $blurup_n/"
fi

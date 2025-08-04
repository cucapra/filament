#!/usr/bin/env bash

# Generate static ALU main module with dependencies
# Usage: ./generate_static_main.sh <adder_stages> <mult_stages>

set -e

if [ $# -ne 2 ]; then
    echo "Usage: $0 <adder_stages> <mult_stages>"
    echo "Example: $0 2 3"
    exit 1
fi

ADD_STAGES=$1
MULT_STAGES=$2

echo "=== Generating Static ALU Main Module ==="
echo "Adder stages: $ADD_STAGES, Multiplier stages: $MULT_STAGES"

# Create _debug directory if it doesn't exist
mkdir -p _debug

# Output file
OUTPUT_FILE="_debug/static_main_${ADD_STAGES}_${MULT_STAGES}.sv"

# Start with header
cat > "$OUTPUT_FILE" << EOF
// Generated Static ALU Main Module with $ADD_STAGES adder stages and $MULT_STAGES multiplier stages

EOF

# Add all adder stage modules
for stage in 1 2 3 4; do
    echo "// FP Adder ${stage}-stage implementation" >> "$OUTPUT_FILE"
    cat "add/fp_adder_${stage}stage.sv" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

# Add all multiplier stage modules  
for stage in 1 2 3 4; do
    echo "// FP Multiplier ${stage}-stage implementation" >> "$OUTPUT_FILE"
    cat "mul/fp_mult_${stage}stage.sv" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

# Add wrapper modules (non-LI versions for static ALU)
echo "// FP Adder Wrapper" >> "$OUTPUT_FILE"
cat "add/fp_adder_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "// FP Multiplier Wrapper" >> "$OUTPUT_FILE"
cat "mul/fp_mult_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Add Static ALU
echo "// Static ALU Implementation" >> "$OUTPUT_FILE"
cat "static_alu.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Create main module that instantiates Static_ALU
cat >> "$OUTPUT_FILE" << EOF
// Main module wrapper for Static ALU
module main #(
    parameter ADDER_STAGES = $ADD_STAGES,
    parameter MULT_STAGES = $MULT_STAGES
) (
    input clk,
    input reset,
    input [31:0] operand_a,
    input [31:0] operand_b,
    input [1:0] operation,
    output [31:0] result,
    output exception,
    output overflow,
    output underflow,
    output valid
);

Static_ALU #(
    .ADDER_STAGES(ADDER_STAGES),
    .MULT_STAGES(MULT_STAGES)
) alu_inst (
    .clk(clk),
    .reset(reset),
    .operand_a(operand_a),
    .operand_b(operand_b),
    .operation(operation),
    .result(result),
    .exception(exception),
    .overflow(overflow),
    .underflow(underflow),
    .valid(valid)
);

endmodule
EOF

echo "Generated: $OUTPUT_FILE"
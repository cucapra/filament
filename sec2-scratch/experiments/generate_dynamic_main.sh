#!/usr/bin/env bash

# Generate dynamic ALU main module with dependencies
# Usage: ./generate_dynamic_main.sh <adder_stages> <mult_stages>

set -e

if [ $# -ne 2 ]; then
    echo "Usage: $0 <adder_stages> <mult_stages>"
    echo "Example: $0 2 3"
    exit 1
fi

ADD_STAGES=$1
MULT_STAGES=$2

echo "=== Generating Dynamic ALU Main Module ==="
echo "Adder stages: $ADD_STAGES, Multiplier stages: $MULT_STAGES"

# Create _debug directory if it doesn't exist
mkdir -p _debug

# Output file
OUTPUT_FILE="_debug/dynamic_main_${ADD_STAGES}_${MULT_STAGES}.sv"

# Start with header
cat > "$OUTPUT_FILE" << EOF
// Generated Dynamic ALU Main Module with $ADD_STAGES adder stages and $MULT_STAGES multiplier stages

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

# Add wrapper modules
echo "// FP Adder Wrapper" >> "$OUTPUT_FILE"
cat "add/fp_adder_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "// FP Multiplier Wrapper" >> "$OUTPUT_FILE"
cat "mul/fp_mult_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "// FP Adder LI Wrapper" >> "$OUTPUT_FILE"
cat "add/fp_adder_li_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "// FP Multiplier LI Wrapper" >> "$OUTPUT_FILE"
cat "mul/fp_mult_li_wrapper.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Add Dynamic ALU
echo "// Dynamic ALU Implementation" >> "$OUTPUT_FILE"
cat "dynamic_alu.sv" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Create main module that instantiates Dynamic_ALU
cat >> "$OUTPUT_FILE" << EOF
// Main module wrapper for Dynamic ALU
module main #(
    parameter ADDER_STAGES = $ADD_STAGES,
    parameter MULT_STAGES = $MULT_STAGES
) (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] operand_a,
    input [31:0] operand_b,
    input [1:0] operation,
    input valid_in,
    output ready_out,
    
    // Output interface
    output [31:0] result,
    output exception,
    output overflow,
    output underflow,
    output valid_out,
    input ready_in
);

Dynamic_ALU #(
    .ADDER_STAGES(ADDER_STAGES),
    .MULT_STAGES(MULT_STAGES)
) alu_inst (
    .clk(clk),
    .reset(reset),
    .operand_a(operand_a),
    .operand_b(operand_b),
    .operation(operation),
    .valid_in(valid_in),
    .ready_out(ready_out),
    .result(result),
    .exception(exception),
    .overflow(overflow),
    .underflow(underflow),
    .valid_out(valid_out),
    .ready_in(ready_in)
);

endmodule
EOF

echo "Generated: $OUTPUT_FILE"
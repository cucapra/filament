#!/usr/bin/env bash

# Synthesis experiment script
# Usage: ./experiment.sh <static|dynamic> <add_stages> <mult_stages>

set -euf -o pipefail

if [ $# -ne 3 ]; then
    echo "Usage: $0 <static|dynamic> <add_stages> <mult_stages>"
    echo "Example: $0 static 2 3"
    exit 1
fi

TYPE=$1
ADD_STAGES=$2
MULT_STAGES=$3

# Validate arguments
if [[ "$TYPE" != "static" && "$TYPE" != "dynamic" ]]; then
    echo "Error: First argument must be 'static' or 'dynamic'"
    exit 1
fi

if ! [[ "$ADD_STAGES" =~ ^[1-4]$ ]]; then
    echo "Error: Add stages must be between 1 and 4"
    exit 1
fi

if ! [[ "$MULT_STAGES" =~ ^[1-4]$ ]]; then
    echo "Error: Mult stages must be between 1 and 4"
    exit 1
fi

echo "=== Starting synthesis experiment ==="
echo "Type: $TYPE, Add stages: $ADD_STAGES, Mult stages: $MULT_STAGES"

# Create experiment directory
EXP_DIR="${TYPE}-${ADD_STAGES}-${MULT_STAGES}"
echo "Creating experiment directory: $EXP_DIR"
mkdir -p "$EXP_DIR"
cd "$EXP_DIR"

# Generate Verilog file (run from parent directory)
echo "=== Generating Verilog file ==="
cd ..
if [ "$TYPE" = "static" ]; then
    ./generate_static_main.sh "$ADD_STAGES" "$MULT_STAGES"
    VERILOG_FILE="${EXP_DIR}/_debug/static_main_${ADD_STAGES}_${MULT_STAGES}.sv"
else
    ./generate_dynamic_main.sh "$ADD_STAGES" "$MULT_STAGES"
    VERILOG_FILE="${EXP_DIR}/_debug/dynamic_main_${ADD_STAGES}_${MULT_STAGES}.sv"
fi

# Copy generated file to experiment directory
cp "_debug/${TYPE}_main_${ADD_STAGES}_${MULT_STAGES}.sv" "$EXP_DIR/"
cd "$EXP_DIR"
VERILOG_FILE="${TYPE}_main_${ADD_STAGES}_${MULT_STAGES}.sv"

# Check if Verilog file was generated
if [ ! -f "$VERILOG_FILE" ]; then
    echo "Error: Verilog file $VERILOG_FILE was not generated"
    exit 1
fi

echo "Generated Verilog file: $VERILOG_FILE"

# Lint with Verilator
echo "=== Linting with Verilator ==="
verilator -Wno-WIDTHTRUNC -Wno-WIDTHEXPAND -Wno-DECLFILENAME -Wno-UNUSEDSIGNAL --lint-only "$VERILOG_FILE"
if [ $? -eq 0 ]; then
    echo "Verilator linting passed"
else
    echo "Error: Verilator linting failed"
    exit 1
fi

# Run synthesis
echo "=== Running synthesis ==="
OUT_DIR="out"
LOG_FILE="synthesis.log"

fud e --to synth-files --from synth-verilog -vv "$VERILOG_FILE" \
    -s synth-verilog.tcl ../synth/synth.tcl \
    -s synth-verilog.constraints ../synth/device.xdc \
    -o "$OUT_DIR" 2> "$LOG_FILE"

# Check if synthesis succeeded
if [ ! -d "$OUT_DIR" ]; then
    echo "Error: Output directory $OUT_DIR was not created"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

# Check for RPT files
echo "=== Verifying RPT files were generated ==="
RPT_COUNT=$(find "$OUT_DIR" -name "*.rpt" | wc -l)
echo "Found $RPT_COUNT RPT files"

if [ "$RPT_COUNT" -eq 0 ]; then
    echo "Error: No RPT files found"
    echo "Last 20 lines of log:"
    tail -20 "$LOG_FILE"
    exit 1
fi

# Find the actual output directory (has tmp.* pattern)
ACTUAL_OUT_DIR=$(find "$OUT_DIR" -type d -name "tmp.*" | head -1)
if [ -z "$ACTUAL_OUT_DIR" ]; then
    echo "Error: Could not find tmp.* directory in $OUT_DIR"
    exit 1
fi

FULL_OUT_DIR="$ACTUAL_OUT_DIR/out"
echo "Using output directory: $FULL_OUT_DIR"

# Generate summary.json
echo "=== Generating utilization summary ==="
synthrep summary -d "$FULL_OUT_DIR" > summary.json
echo "Generated summary.json"

# Generate hierarchy.json  
echo "=== Generating hierarchy summary ==="
synthrep summary -m hierarchy -d "$FULL_OUT_DIR" > hierarchy.json
echo "Generated hierarchy.json"

# Generate timing.json
echo "=== Generating timing analysis ==="
TIMING_RPT=$(find "$FULL_OUT_DIR" -name "main_timing_summary_routed.rpt" | head -1)
if [ -n "$TIMING_RPT" ]; then
    python3 ../../timing-analysis/parse.py "$TIMING_RPT" > timing.json
    echo "Generated timing.json"
else
    echo "Warning: No timing summary report found"
fi

# Check timing status
echo "=== Final Summary ==="
echo "Experiment: $TYPE ALU with $ADD_STAGES adder stages and $MULT_STAGES multiplier stages"
echo "Directory: $(pwd)"

if [ -f "summary.json" ]; then
    TIMING_MET=$(jq -r '.meet_timing // "unknown"' summary.json)
    WORST_SLACK=$(jq -r '.worst_slack // "unknown"' summary.json)
    
    echo "Timing met: $TIMING_MET"
    echo "Worst slack: $WORST_SLACK"
else
    echo "Warning: Could not read timing information from summary.json"
fi

echo ""
echo "Generated files:"
echo "  - Verilog: $VERILOG_FILE"
echo "  - Synthesis log: $LOG_FILE"
if [ -f "summary.json" ]; then
    echo "  - Utilization summary: $(pwd)/summary.json"
fi
if [ -f "hierarchy.json" ]; then
    echo "  - Hierarchy summary: $(pwd)/hierarchy.json"  
fi
if [ -f "timing.json" ]; then
    echo "  - Timing analysis: $(pwd)/timing.json"
fi

echo "=== Experiment completed successfully ==="

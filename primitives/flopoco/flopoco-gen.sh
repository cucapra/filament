#!/bin/bash

set -euf -o pipefail

# Check if the flopoco binary is exists
if ! command -v flopoco &> /dev/null
then
    echo "flopoco binary could not be found"
    exit
fi
if ! command -v ghdl &> /dev/null
then
    echo "ghdl binary could not be found"
    exit
fi
if ! command -v verilator &> /dev/null
then
    echo "verilator binary could not be found"
    exit
fi

# The location of the script. The vhdl file is always generated here because of
# the docker command.
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# Crash if less than three arguments are provided
if [ "$#" -lt 3 ]; then
    echo "Usage: $0 <name> <output> <args>"
    exit 1
fi

# Name of the generated module
NAME="$1"

# Output directory for the file
OUT="$2"

# The remaining arguments are passed as is
shift
shift
ARGS="$@"

>&2 echo "Generating $NAME in $OUT"

FLOPOCO="$(dirname "$OUT")/flopoco.vhdl"
# Generate the module
flopoco $ARGS name=$NAME outputFile=$FLOPOCO 2> >(tee "$OUT.err" >&2)

# Transform the generated VHDL to Verilog
ghdl synth --out=verilog -fsynopsys -fexplicit \
 $FLOPOCO -e "$NAME" > "$OUT"

# In-place update the generated Verilog to use `\table` instead of `table`
# sed 's/[[:<:]]table[[:>:]]/\\table/g' "$OUT" > "$OUT.tmp"
cat "$OUT" > "$OUT.tmp"
mv "$OUT.tmp" "$OUT"

# Check that generated Verilog is valid
verilator --lint-only "$OUT"

# Find the last occurence of "Pipeline depth = n" in the error log and report n as the latency
set +o pipefail
LATENCY=$(grep -n "Pipeline depth = " "$OUT.err" | tail -1 | cut -d' ' -f7)
set -o pipefail
# If the pipeline depth is not found, set it to 0
if [ -z "$LATENCY" ]; then
    LATENCY=0
fi
echo "pipeline_depth = $LATENCY"
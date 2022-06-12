#!/bin/sh

set -euf -o pipefail

base="$1"

# Move to the directory of the script
cd "$(dirname "$0")"

# Compile design to Calyx
"$base/target/debug/filament" --calyx-primitives "$base/../calyx" "lowered.fil" > "lowered.futil" 2> /dev/null

# Generate Verilog from Calyx
fud e "lowered.futil" --to icarus-verilog -o "out.sv" -s futil.flags ' -d papercut' -q

# Compile & run using iverilog
out="a.out"
iverilog -g2012 -o "$out" "tb.sv"
"./$out" > /dev/null

# Get a JSON representation of the final VCD
fud e "dump.vcd" --to jq -s jq.expr '.TOP.main.M.out | unique' -q

rm lowered.futil out.sv a.out

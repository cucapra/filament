#!/bin/bash

set -euf -o pipefail


dir="$1"
# Number of data points to test out
count="$2"
data="$dir/data.json"

# Location of data generator
base="${BASH_SOURCE%/*}"
script_dir="$base/../evaluation/fuzz"

# Generate random data
python3 "$script_dir/gen_float.py" gen --fields a0 b0 a1 b1 a2 b2 c --width 8 "$count" > "$data"

# Run the data through the harness
(fud e -s cocotb.data "$data" --to cocotb-out "$dir/harness.fil" -s futil.flags ' -d canonicalize' -q | \
  "$script_dir/gen_float.py" check --fields gold y && \
  echo "No counterexamples with $count data points" && rm "$data") || \
  cat "$data"
#!/bin/bash

set -euf -o pipefail

# Location to the directory that contains harness.fil
dir="$1"
# Number of data points to test out
count="$2"

# Directory that contains this script
script_dir="${BASH_SOURCE%/*}"

# Data file
data=$"$dir/data.json"
# Fields file
fields=$"$dir/fields"
# Width file
width=$"$dir/width"
# Operator file
op=$"$dir/op"

./"$script_dir/gen_float.py" gen --width $(cat $width) --op $(cat $op) "$count" > "$data"

(fud e -s cocotb.data "$data" --to cocotb-out "$dir/harness.fil" -s calyx.flags ' -d canonicalize' -q | \
  ./"$script_dir/gen_float.py" check --fields $(cat $fields) && \
  echo "No counterexamples with $count data points" && rm "$data") || \
  cat "$data"
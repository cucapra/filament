#!/bin/bash

set -euf -o pipefail

# Location to the directory that contains harness.fil
dir="$1"

# Directory that contains this script
script_dir="${BASH_SOURCE%/*}"

# Data file
data=$"$dir/$2.json"
# Fields file
fields=$"$dir/fields"
# Width file
width=$"$dir/width"

(fud e -s cocotb.data "$data" --to cocotb-out "$dir/harness.fil" -s calyx.flags ' -d canonicalize' -q | \
  ./"$script_dir/gen_float.py" check --fields $(cat $fields) && \
  echo "No counterexamples found") || \
  (cat "$data"; exit 1)
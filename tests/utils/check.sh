#!/bin/bash

set -euf -o pipefail

# Location to the directory that contains harness.fil
dir="$1"

# Directory that contains this script
script_dir="${BASH_SOURCE%/*}"

# Data file
data=$"$dir/$2.json"
# Input fields file
infields=$"$dir/params/in"
# Output fields file
outfields=$"$dir/params/out"
# Width file
width=$"$dir/params/width"
# Dtype file
dtype=$"$dir/params/dtype"
# Epsilon file
eps=$"$dir/params/epsilon"

(fud e -s cocotb.data "$data" --to cocotb-out "$dir/harness.fil" -s calyx.flags ' -d canonicalize' -q | \
  ./"$script_dir/gen_float.py" check --fields $(cat $outfields) --dt $(cat $dtype) --epsilon $(cat $eps) --width $(cat $width) && \
  echo "No counterexamples found") || \
  (cat "$data"; exit 1)

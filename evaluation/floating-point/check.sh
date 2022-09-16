#!/bin/bash

set -euf -o pipefail

# Location to the directory that contains harness.fil
dir="$1"
# Name of the operation implemented by harness.fil
op="$2"
# Number of data points to test out
count="$3"

# Directory that contains this script
script_dir="${BASH_SOURCE%/*}"

# Data file
data=$"$dir/data.json"
# Fields file
fields=$"$dir/fields"

./"$script_dir/gen_float.py" gen --raw "$count" --op "$op" > "$data"

fud e -s cocotb.data "$data" --to cocotb-out "$dir/harness.fil" -s futil.flags ' -d canonicalize' -q | \
  ./"$script_dir/gen_float.py" to_float | \
  ./"$script_dir/gen_float.py" check --fields $(cat $fields)

# If the command failed, print out the data file
if [ $? -ne 0 ]; then
  cat "$data"
else
  echo "No counterexamples with $count data points"
  rm "$data"
fi
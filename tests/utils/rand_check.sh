#!/bin/bash

set -euf -o pipefail

# Location to the directory that contains harness.fil
dir="$1"
# Number of data points to test out
count="$2"

# Directory that contains this script
script_dir="${BASH_SOURCE%/*}"

# Data file
data=$"$dir/rand.json"
# Fields file
fields=$"$dir/fields"
# Width file
width=$"$dir/width"

./"$script_dir/gen_float.py" gen --width $(cat $width) "$count" > "$data"

./"$script_dir/check.sh" $dir rand && rm -f "$data"
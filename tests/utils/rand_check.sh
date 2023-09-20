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
# Input fields file
infields=$"$dir/params/in"
# Width file
width=$"$dir/params/width"
# Dtype file
dtype=$"$dir/params/dtype"
# Bound file
bound=$"$dir/params/bound"

./"$script_dir/gen_float.py" gen --width $(cat $width) --fields $(cat $infields) --dt $(cat $dtype) --bound $(cat $bound) --count "$count" > "$data"

./"$script_dir/check.sh" $dir rand && rm -f "$data"
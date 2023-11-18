#!/bin/bash

WORKDIR="$(pwd)"

# path to .fil
SRC="$1"

# destination directory for reports
DST="$2"

# mkdir $WORKDIR/$DST/tmp
# cargo run -- $WORKDIR/$SRC > $WORKDIR/$DST/tmp/tmp.sv
cd $WORKDIR/$DST
fud e -vv --from synth-verilog $WORKDIR/$DST/impl.sv --to synth-files -o report -s synth-verilog.remote 1
fud e --from synth-files --to resource-estimate report > rpt_parsed.json
cd $WORKDIR/$DST


#!/bin/bash

WORKDIR="$(pwd)"

# path to .fil
SRC="$1"

# destination directory for reports
DST="$2"

cargo run -- $WORKDIR/$SRC --log info > $WORKDIR/$DST/impl.sv
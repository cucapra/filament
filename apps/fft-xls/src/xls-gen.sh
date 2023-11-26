#!/bin/bash

set -euf -o pipefail

# location of the script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# path to xls download
# XLS_PATH="$1"

# name of generated module
NAME="$1"

# name of file, located in this folder
FILE_NAME="$2"

# output path of generated files
OUT="$3"

# exponent width
# EXP_WIDTH="$4"

# mantissa width
# MANT_WIDTH="$5"

# name of func to compile
FUNC_NAME="$4"

# number of pipeline stages
STAGES="$5"

PREFIX="$( cd "$( dirname "$OUT" )" >/dev/null 2>&1 && pwd )"
XLS_REPL="$PREFIX/xls.x"
XLS_IR="$PREFIX/xls.ir"
XLS_OPT_IR="$PREFIX/xls.opt.ir"
XLS_V="$PREFIX/xls.sv"
TMP="$PREFIX/tmp"

OUT_V="$PREFIX/$OUT"

>&2 echo "Generating $NAME in $OUT"

# sed "s/FSIZE/$MANT_WIDTH/g" $DIR/$FILE_NAME > $XLS_REPL
# sed -i "s/ESIZE/$EXP_WIDTH/g" $XLS_REPL

# cd "$XLS_PATH"
set +x
cd /home/xls
./bazel-bin/xls/dslx/ir_convert/ir_converter_main $DIR/$FILE_NAME > $XLS_IR --emit_fail_as_assert=false
./bazel-bin/xls/tools/opt_main --top=__fft__"$FUNC_NAME" $XLS_IR > $XLS_OPT_IR

./bazel-bin/xls/tools/codegen_main $XLS_OPT_IR --top __fft__"$FUNC_NAME" --generator=pipeline --delay_model="unit" --output_verilog_path=$OUT \
    --module_name="$NAME" --pipeline_stages="$STAGES" --use_system_verilog

# verilator --lint-only "$OUT"

echo "pipeline_depth = $STAGES"
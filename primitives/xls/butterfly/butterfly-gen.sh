#!/bin/bash

set -euf -o pipefail

# location of the script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# path to xls download
XLS_PATH="$1"

# name of generated module
NAME="$2"

# name of file, located in this folder
FILE_NAME="$3"

# output path of generated files
OUT="$4"

# exponent width
EXP_WIDTH="$5"

# mantissa width
MANT_WIDTH="$6"

# name of func to compile
FUNC_NAME="$7"

# number of pipeline stages
STAGES="$8"

PREFIX="$( cd "$( dirname "$OUT" )" >/dev/null 2>&1 && pwd )"
XLS_REPL="$PREFIX/xls.x"
XLS_IR="$PREFIX/xls.ir"
XLS_OPT_IR="$PREFIX/xls.opt.ir"
XLS_V="$PREFIX/xls.sv"
TMP="$PREFIX/tmp"

>&2 echo "Generating $NAME in $OUT"


sed "s/FSIZE/$MANT_WIDTH/g" $DIR/$FILE_NAME > $XLS_REPL
sed -i "s/ESIZE/$EXP_WIDTH/g" $XLS_REPL

cd "$XLS_PATH"
./bazel-bin/xls/dslx/ir_convert/ir_converter_main $XLS_REPL > $XLS_IR --emit_fail_as_assert=false
./bazel-bin/xls/tools/opt_main --top=__xls__"$FUNC_NAME" $XLS_IR > $XLS_OPT_IR

./bazel-bin/xls/tools/codegen_main $XLS_OPT_IR --top __xls__"$FUNC_NAME" --generator=pipeline --delay_model="unit" --output_verilog_path=$XLS_V \
    --module_name="$NAME"_Exp"$EXP_WIDTH"_Mant"$MANT_WIDTH" --pipeline_stages="$STAGES" --use_system_verilog

verilator --lint-only "$XLS_V"

echo "pipeline_depth = $STAGES"
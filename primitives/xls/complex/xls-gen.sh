#!/bin/bash

set -euf -o pipefail

# location of the script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# name of generated module
NAME="$1"

# absolute path to DSLX file
SRC_PATH="$2"

FILE_NAME="$3"

# output path of generated files
DEST_PATH="$4"

# exponent width
EXP_WIDTH="$5"

# mantissa width
MANT_WIDTH="$6"

# name of func to compile
FUNC_NAME="$7"

# number of pipeline stages
STAGES="$8"

mkdir "$DEST_PATH"/tmp
cd "$DEST_PATH"/tmp
sed "s/FRACTION_SIZE/$MANT_WIDTH/g" $SRC_PATH/$FILE_NAME > $DEST_PATH/tmp/tmp.x
sed -i "s/EXP_SIZE/$EXP_WIDTH/g" $DEST_PATH/tmp/tmp.x

cd /home/xls/
./bazel-bin/xls/dslx/ir_convert/ir_converter_main $DEST_PATH/tmp/tmp.x > $DEST_PATH/tmp/impl.ir
./bazel-bin/xls/tools/opt_main --top=__tmp__"$FUNC_NAME" $DEST_PATH/tmp/impl.ir > $DEST_PATH/tmp/impl.opt.ir

./bazel-bin/xls/tools/codegen_main $DEST_PATH/tmp/impl.opt.ir --top __tmp__"$FUNC_NAME" --generator=pipeline --delay_model="unit" --output_verilog_path=$DEST_PATH/impl.v \
    --module_name="$NAME"_Exp"$EXP_WIDTH"_Mant"$MANT_WIDTH" --pipeline_stages="$STAGES" --use_system_verilog

rm -rf "$DEST_PATH/tmp"
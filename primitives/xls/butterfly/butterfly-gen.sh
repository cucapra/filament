#!/bin/bash

set -euf -o pipefail

# location of the script
# DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
DIR="$(pwd)"

# name of generated module
NAME="$1"

# absolute path to DSLX file
SRC_PATH="$2"

# output path of generated files
DEST_PATH="$3"

# exponent width
EXP_WIDTH="$4"

# mantissa width
MANT_WIDTH="$5"

# name of func to compile
FUNC_NAME="$6"

# number of pipeline stages
STAGES="$7"

mkdir "$DEST_PATH"/tmp
cd "$DEST_PATH"/tmp
sed "s/FSIZE/$MANT_WIDTH/g" $DIR/$SRC_PATH > $DIR/$DEST_PATH/tmp/tmp.x
sed -i "s/ESIZE/$EXP_WIDTH/g" $DIR/$DEST_PATH/tmp/tmp.x

cd /scratch/ehg54/xls/
./bazel-bin/xls/dslx/ir_convert/ir_converter_main $DIR/$DEST_PATH/tmp/tmp.x > $DIR/$DEST_PATH/tmp/impl.ir
./bazel-bin/xls/tools/opt_main --top=__tmp__"$FUNC_NAME" $DIR/$DEST_PATH/tmp/impl.ir > $DIR/$DEST_PATH/tmp/impl.opt.ir

./bazel-bin/xls/tools/codegen_main $DIR/$DEST_PATH/tmp/impl.opt.ir --top __tmp__"$FUNC_NAME" --generator=pipeline --delay_model="unit" --output_verilog_path=$DIR/$DEST_PATH/impl.v \
    --module_name="$NAME"_Exp"$EXP_WIDTH"_Mant"$MANT_WIDTH" --pipeline_stages="$STAGES" --use_system_verilog

rm -rf "$DIR/$DEST_PATH/tmp"
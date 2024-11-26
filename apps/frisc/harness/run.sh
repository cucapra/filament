#!/bin/bash

# Script that will compile the RISC-V core, run the Calyx harness, and then 
# delete generated files

cargo run --bin filament ../src/cpu.fil --library ../../.. --backend calyx --disable-emit-toplevel --preserve-names > frisc.futil

python expected.py

rm top.err
rm top.out
rm top.json
rm frisc.futil

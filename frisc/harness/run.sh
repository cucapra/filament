#!/bin/bash

# Script that will compile the RISC-V core, run the Calyx harness, and then 
# delete generated files

cargo run -- ../src/cpu.fil --library ../.. --toplevel CPU > frisc.futil

fud exec top.futil --to dat --through icarus-verilog -s verilog.data top.json \
  --from futil

#rm frisc.futil

rm model.smt
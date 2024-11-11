#!/bin/bash

# Script that will compile the RISC-V core, run the Calyx harness, and then 
# delete generated files

cargo run --bin filament ../src/cpu.fil --library ../../.. > frisc.futil

fud exec top.futil --to dat --through icarus-verilog -s verilog.data top.json \
  --from calyx

# rm frisc.futil

rm model.smt
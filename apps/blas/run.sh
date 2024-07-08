#!/bin/bash

KERNEL=$1
fud e --to cocotb-out apps/blas/$KERNEL/test.fil -s cocotb.data apps/blas/$KERNEL/test.fil.data -s calyx.flags ' -d canonicalize'
#!/bin/bash

fud exec --to cocotb-out apps/xls/attempt.fil -s cocotb.data apps/xls/data.json -s calyx.flags ' -d canonicalize' -s filament.flags ' --solver z3'
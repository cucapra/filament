#!/usr/bin/env python3

from pyosys import libyosys as ys

import matplotlib.pyplot as plt
import numpy as np
import argparse

design = ys.Design()
ys.run_pass("read_verilog -nosynthesis ../../tests/simple/fiedler-cooley.v", design)

cell_stats = {}
for module in design.selected_whole_modules_warn():
    for cell in module.selected_cells():
        if cell.type.str() in cell_stats:
            cell_stats[cell.type.str()] += 1
        else:
            cell_stats[cell.type.str()] = 1
plt.bar(range(len(cell_stats)), height=list(cell_stats.values()), align="center")
plt.xticks(range(len(cell_stats)), list(cell_stats.keys()))
plt.show()


def main(file: str):
    design = ys.Design()
    ys.run_pass(f"read_verilog -nosynthesis {file}", design)

    cell_stats = {}
    for module in design.selected_whole_modules_warn():
        for cell in module.selected_cells():
            if cell.type.str() in cell_stats:
                cell_stats[cell.type.str()] += 1
            else:
                cell_stats[cell.type.str()] = 1
    plt.bar(range(len(cell_stats)), height=list(cell_stats.values()), align="center")
    plt.xticks(range(len(cell_stats)), list(cell_stats.keys()))
    plt.show()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Prefix verilog modules")
    parser.add_argument("file", type=str, help="Input file")
    parser.add_argument("-o", "--output", type=str, help="Output file", default=None)
    args = parser.parse_args()
    main(args.file)

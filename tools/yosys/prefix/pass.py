#!/usr/bin/env python3

from pyosys import libyosys as ys

import matplotlib.pyplot as plt
import numpy as np


class PrefixModulePass(ys.Pass):
    def __init__(self):
        super().__init__("cell_stats", "Shows cell stats as plot")

    def py_help(self):
        ys.log("This module prefixes module names.\n")

    def py_execute(self, args, design):
        ys.log_header(design, "Prefixing Module Names\n")
        cell_stats = {}
        for module in design.selected_whole_modules_warn():
            for cell in module.selected_cells():
                if cell.type.str() in cell_stats:
                    cell_stats[cell.type.str()] += 1
                else:
                    cell_stats[cell.type.str()] = 1
        plt.bar(
            range(len(cell_stats)), height=list(cell_stats.values()), align="center"
        )
        plt.xticks(range(len(cell_stats)), list(cell_stats.keys()))
        plt.show()

    def py_clear_flags(self):
        ys.log("Clear Flags - PrefixModulePass\n")

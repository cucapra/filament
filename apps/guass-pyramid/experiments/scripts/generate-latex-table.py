#!/usr/bin/env python3

import json
import sys

def generate_latex_table(json_file='result.json'):
    """Generate a LaTeX table from result.json with booktabs formatting."""

    # Read the JSON data
    with open(json_file, 'r') as f:
        data = json.load(f)

    # Extract design names and sort by II value
    designs = sorted(data['ii'].keys(), key=lambda x: data['ii'][x])

    # Start building the LaTeX table
    lines = []
    lines.append(r'\begin{tabular}{lccc}')
    lines.append(r'\toprule')
    lines.append(r'\textbf{Design} & \textbf{LUTs} & \textbf{Registers} & \textbf{II} \\')
    lines.append(r'\midrule')

    # Add data rows
    for design in designs:
        luts = data['luts'][design]
        registers = data['registers'][design]
        ii = data['ii'][design]
        lines.append(f'{design} & {luts} & {registers} & {ii} \\\\')

    lines.append(r'\bottomrule')
    lines.append(r'\end{tabular}')

    # Print the table
    for line in lines:
        print(line)

if __name__ == '__main__':
    # Allow optional json file argument
    json_file = sys.argv[1] if len(sys.argv) > 1 else 'result.json'
    generate_latex_table(json_file)

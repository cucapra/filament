#!/usr/bin/env python3

import json
import sys
import re
import argparse

def format_filament_design(design_path):
    """Format filament design names from fil-*/blur_{ii}-out to Filament ({ii})."""
    match = re.search(r'fil-[^/]*/blur_(\d+)-out', design_path)
    if match:
        ii = match.group(1)
        return f'Lilac ({ii})'
    return design_path

def format_rv_design(design_path):
    """Format ready-valid design names from rv-*/rv-{ii}-out to RV ({ii})."""
    match = re.search(r'rv-[^/]*/rv-(\d+)-out', design_path)
    if match:
        ii = match.group(1)
        return f'RV ({ii})'
    return design_path

def format_design_name(design_path):
    """Format design names based on their pattern."""
    if 'fil-' in design_path and 'blur_' in design_path:
        return format_filament_design(design_path)
    elif 'rv-' in design_path and 'rv-' in design_path.split('/')[-1]:
        return format_rv_design(design_path)
    return design_path

def extract_filament_latencies(file_path):
    """Extract filament latencies and create II to latency mapping."""
    with open(file_path, 'r') as f:
        data = json.load(f)

    # Create mapping from II value to latency
    ii_to_latency = {}

    # The file has format: { "latency": { "blur_X": value }, "ii": { "blur_X": value } }
    for design, latency in data['latency'].items():
        # Extract II value from blur_X pattern
        match = re.search(r'blur_(\d+)', design)
        if match:
            ii = int(match.group(1))
            ii_to_latency[ii] = latency

    return ii_to_latency

def extract_rv_latencies(file_path):
    """Extract RV latencies from pyramid-cycles.json and create II to latency mapping."""
    with open(file_path, 'r') as f:
        data = json.load(f)

    # Create mapping from II value to latency
    ii_to_latency = {}

    # The file has format: { "pyramid_nX": value }
    for design, latency in data.items():
        # Extract II value from pyramid_nX pattern
        match = re.search(r'pyramid_n(\d+)', design)
        if match:
            ii = int(match.group(1))
            ii_to_latency[ii] = latency

    return ii_to_latency

def generate_latex_table(results_file, filament_latencies_file, rv_latencies_file):
    """Generate a LaTeX table from results.json with booktabs formatting."""

    # Read the results data
    with open(results_file, 'r') as f:
        data = json.load(f)

    # Extract latency mappings
    filament_latencies = extract_filament_latencies(filament_latencies_file)
    rv_latencies = extract_rv_latencies(rv_latencies_file)

    # Extract design names and sort by type and II value
    def sort_key(design):
        formatted = format_design_name(design)
        if formatted.startswith('Lilac'):
            # Extract II value for filament designs
            match = re.search(r'Lilac \((\d+)\)', formatted)
            ii = int(match.group(1)) if match else 0
            return (0, ii)  # Filament first, then by II
        elif formatted.startswith('RV'):
            # Extract II value for RV designs
            match = re.search(r'RV \((\d+)\)', formatted)
            ii = int(match.group(1)) if match else 0
            return (1, ii)  # RV second, then by II
        else:
            return (2, design)  # Other designs last, alphabetically

    designs = sorted(data['luts'].keys(), key=sort_key)

    # Start building the LaTeX table
    lines = []
    lines.append(r'\begin{tabular}{lcccc}')
    lines.append(r'\toprule')
    lines.append(r'\textbf{Design} & \textbf{LUTs} & \textbf{Registers} & \textbf{Latency} & \textbf{Freq. (MHz)} \\')
    lines.append(r'\midrule')

    # Add data rows
    for design in designs:
        formatted_name = format_design_name(design)
        luts = data['luts'][design]
        registers = data['registers'][design]
        freq = data['possible_freq'][design]

        # Get latency based on design type and II value
        latency = ""
        if formatted_name.startswith('Lilac'):
            # Extract II value for filament designs
            match = re.search(r'Lilac \((\d+)\)', formatted_name)
            if match:
                ii = int(match.group(1))
                latency = filament_latencies.get(ii, "N/A")
        elif formatted_name.startswith('RV'):
            # Extract II value for RV designs
            match = re.search(r'RV \((\d+)\)', formatted_name)
            if match:
                ii = int(match.group(1))
                latency = rv_latencies.get(ii, "N/A")

        lines.append(f'{formatted_name} & {luts} & {registers} & {latency} & {freq} \\\\')

    lines.append(r'\bottomrule')
    lines.append(r'\end{tabular}')

    # Print the table
    for line in lines:
        print(line)

def main():
    parser = argparse.ArgumentParser(description='Generate LaTeX table from synthesis results and latency data')
    parser.add_argument('--results', required=True,
                        help='Path to results.json file with resource utilization data')
    parser.add_argument('--fil-lat', required=True,
                        help='Path to filament latencies JSON file')
    parser.add_argument('--rv-lat', required=True,
                        help='Path to RV latencies JSON file (pyramid-cycles.json)')

    args = parser.parse_args()

    try:
        generate_latex_table(args.results, args.fil_lat, args.rv_lat)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON file: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()

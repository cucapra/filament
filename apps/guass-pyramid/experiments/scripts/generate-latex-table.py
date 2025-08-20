#!/usr/bin/env python3

import json
import sys
import re
import argparse
import math

RV_NAME = "RV"


def format_filament_design(design_path):
    """Format filament design names from fil-*/blur_{ii}-out to Filament ({ii})."""
    match = re.search(r"fil-[^/]*/blur_(\d+)-out", design_path)
    if match:
        ii = match.group(1)
        return f"Lilac ({ii})"
    return design_path


def format_rv_design(design_path):
    """Format ready-valid design names from rv-*/rv-{ii}-out to RV ({ii})."""
    match = re.search(r"rv-[^/]*/rv-(\d+)-out", design_path)
    if match:
        ii = match.group(1)
        return f"RV ({ii})"
    return design_path


def format_design_name(design_path):
    """Format design names based on their pattern."""
    if "fil-" in design_path and "blur_" in design_path:
        return format_filament_design(design_path)
    elif "rv-" in design_path and "rv-" in design_path.split("/")[-1]:
        return format_rv_design(design_path)
    return design_path


def extract_filament_latencies(file_path):
    """Extract filament latencies and create II to latency mapping."""
    with open(file_path, "r") as f:
        data = json.load(f)

    # Create mapping from II value to latency
    ii_to_latency = {}

    # The file has format: { "latency": { "blur_X": value }, "ii": { "blur_X": value } }
    for design, latency in data["latency"].items():
        # Extract II value from blur_X pattern
        match = re.search(r"blur_(\d+)", design)
        if match:
            ii = int(match.group(1))
            ii_to_latency[ii] = latency

    return ii_to_latency


def extract_rv_latencies(file_path):
    """Extract RV latencies from pyramid-cycles.json and create II to latency mapping."""
    with open(file_path, "r") as f:
        data = json.load(f)

    # Create mapping from II value to latency
    ii_to_latency = {}

    # The file has format: { "pyramid_nX": value }
    for design, latency in data.items():
        # Extract II value from pyramid_nX pattern
        match = re.search(r"pyramid_n(\d+)", design)
        if match:
            ii = int(match.group(1))
            ii_to_latency[ii] = latency

    return ii_to_latency


def collect_design_data(results_file, filament_latencies_file, rv_latencies_file):
    """Collect and structure design data from input files."""

    # Read the results data
    with open(results_file, "r") as f:
        data = json.load(f)

    # Extract latency mappings
    filament_latencies = extract_filament_latencies(filament_latencies_file)
    rv_latencies = extract_rv_latencies(rv_latencies_file)

    # Extract design names and sort by type and II value
    def sort_key(design):
        formatted = format_design_name(design)
        if formatted.startswith("Lilac"):
            # Extract II value for filament designs
            match = re.search(r"Lilac \((\d+)\)", formatted)
            ii = int(match.group(1)) if match else 0
            return (0, ii)  # Filament first, then by II
        elif formatted.startswith("RV"):
            # Extract II value for RV designs
            match = re.search(r"RV \((\d+)\)", formatted)
            ii = int(match.group(1)) if match else 0
            return (1, ii)  # RV second, then by II
        else:
            return (2, design)  # Other designs last, alphabetically

    designs = sorted(data["luts"].keys(), key=sort_key)

    # Collect structured data for each design
    design_data = []
    for design in designs:
        formatted_name = format_design_name(design)
        luts = data["luts"][design]
        registers = data["registers"][design]
        freq = data["possible_freq"][design]

        # Get latency based on design type and II value
        latency = None
        design_type = None
        ii_value = None

        if formatted_name.startswith("Lilac"):
            design_type = "Lilac"
            # Extract II value for filament designs
            match = re.search(r"Lilac \((\d+)\)", formatted_name)
            if match:
                ii_value = int(match.group(1))
                latency = filament_latencies.get(ii_value, None)
        elif formatted_name.startswith("RV"):
            design_type = "RV"
            # Extract II value for RV designs
            match = re.search(r"RV \((\d+)\)", formatted_name)
            if match:
                ii_value = int(match.group(1))
                latency = rv_latencies.get(ii_value, None)

        design_data.append(
            {
                "name": formatted_name,
                "design_type": design_type,
                "ii": ii_value,
                "luts": luts,
                "registers": registers,
                "latency": latency,
                "freq_mhz": freq,
            }
        )

    return design_data


def calculate_geometric_mean(values):
    """Calculate geometric mean of a list of values."""
    if not values or any(v <= 0 for v in values):
        return None
    product = math.prod(values)
    return product ** (1.0 / len(values))


def calculate_resource_ratios(design_data):
    """Calculate Lilac/RV ratios for LUTs and Registers."""
    # Group data by II value
    lilac_data = {}
    rv_data = {}

    for design in design_data:
        ii = design["ii"]
        if design["design_type"] == "Lilac":
            lilac_data[ii] = design
        elif design["design_type"] == "RV":
            rv_data[ii] = design

    # Calculate ratios for matching II values
    lut_ratios = []
    register_ratios = []

    for ii in set(lilac_data.keys()) & set(rv_data.keys()):
        lilac = lilac_data[ii]
        rv = rv_data[ii]

        if rv["luts"] > 0:
            lut_ratios.append(lilac["luts"] / rv["luts"])

        if rv["registers"] > 0:
            register_ratios.append(lilac["registers"] / rv["registers"])

    return lut_ratios, register_ratios


def format_fps(fps):
    """Format frames per second with k/M suffixes."""
    if fps is None or fps == "N/A":
        return "N/A"

    if fps >= 1000000:
        return f"{fps / 1000000:.2f}M"
    elif fps >= 1000:
        return f"{int(fps / 1000)}k"
    else:
        return str(int(fps))


def generate_latex_table(
    results_file, filament_latencies_file, rv_latencies_file, add_fps=False
):
    """Generate a LaTeX table from results.json with booktabs formatting."""

    design_data = collect_design_data(
        results_file, filament_latencies_file, rv_latencies_file
    )

    # Start building the LaTeX table
    lines = []

    # Determine column count and headers
    lines.append(r"\begin{figure}")
    lines.append(r"\begin{tabular}{lccccr}")
    lines.append(r"\toprule")

    if add_fps:
        lines.append(
            r"\textbf{Design} & \textbf{LUTs} & \textbf{Registers} & \textbf{Latency} & \textbf{Freq. (MHz)} & \textbf{Frames/sec} \\"
        )
    else:
        lines.append(
            r"\textbf{Design (N)} & \textbf{LUTs} & \textbf{Registers} & \textbf{Latency} & \textbf{Freq. (MHz)} \\"
        )

    lines.append(r"\midrule")

    # Add data rows
    for design in design_data:
        latency_str = str(design["latency"]) if design["latency"] is not None else "N/A"

        if add_fps:
            # Calculate frames per second
            if design["latency"] is not None and design["latency"] != "N/A":
                fps = (design["freq_mhz"] * 1000000) / design["latency"]
                fps_str = format_fps(fps)
            else:
                fps_str = "N/A"

            lines.append(
                f"\\bench{{{design['name']}}} & {design['luts']} & {design['registers']} & {latency_str} & {design['freq_mhz']} & {fps_str} \\\\"
            )
        else:
            lines.append(
                f"\\bench{{{design['name']}}} & {design['luts']} & {design['registers']} & {latency_str} & {design['freq_mhz']} \\\\"
            )

    lines.append(r"\bottomrule")
    lines.append(r"\end{tabular}")
    lines.append(r"\caption{Resource usage and performance of Blur Pyramid configurations.}\label{tbl:blur-comp}")
    lines.append(r"\end{figure}")

    # Print the table
    for line in lines:
        print(line)

    # Calculate and print resource usage summary
    lut_ratios, register_ratios = calculate_resource_ratios(design_data)

    print("\n% Resource Usage Summary:")
    print("% -----------------------")

    if lut_ratios:
        print(lut_ratios)
        lut_geomean = calculate_geometric_mean(lut_ratios)
        if lut_geomean is not None:
            lut_percentage = lut_geomean * 100
            lut_reduction = (1 - lut_geomean) * 100
            print(f"\\newcommand{{\\blurEvalLutOverhead}}{{{lut_reduction:.1f}\%}}")
            print(
                f"% LUTs: Lilac uses {lut_percentage:.1f}% of RV resources ({lut_reduction:.1f}% reduction)"
            )

    if register_ratios:
        reg_geomean = calculate_geometric_mean(register_ratios)
        if reg_geomean is not None:
            reg_percentage = reg_geomean * 100
            reg_reduction = (1 - reg_geomean) * 100
            print(f"\\newcommand{{\\blurEvalRegOverhead}}{{{reg_reduction:.1f}\%}}")
            print(
                f"% Registers: Lilac uses {reg_percentage:.1f}% of RV resources ({reg_reduction:.1f}% reduction)"
            )


def main():
    parser = argparse.ArgumentParser(
        description="Generate LaTeX table from synthesis results and latency data"
    )
    parser.add_argument(
        "--results",
        required=True,
        help="Path to results.json file with resource utilization data",
    )
    parser.add_argument(
        "--fil-lat", required=True, help="Path to filament latencies JSON file"
    )
    parser.add_argument(
        "--rv-lat",
        required=True,
        help="Path to RV latencies JSON file (pyramid-cycles.json)",
    )
    parser.add_argument(
        "--add-fps", action="store_true", help="Add Frames/sec column to the table"
    )

    args = parser.parse_args()

    try:
        generate_latex_table(args.results, args.fil_lat, args.rv_lat, args.add_fps)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON file: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

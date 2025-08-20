#!/usr/bin/env python3

import json
import sys
import argparse
import importlib.util
import os

# Import from the same directory
script_dir = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("generate_latex_table", os.path.join(script_dir, "generate-latex-table.py"))
generate_latex_table = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generate_latex_table)

collect_design_data = generate_latex_table.collect_design_data

def generate_json_table(results_file, filament_latencies_file, rv_latencies_file, add_fps=False):
    """Generate a JSON table from synthesis results and latency data."""
    
    design_data = collect_design_data(results_file, filament_latencies_file, rv_latencies_file)
    
    # Structure data by design type and II value
    output = {}
    
    for design in design_data:
        design_type = design['design_type']
        ii_value = design['ii']
        
        if design_type not in output:
            output[design_type] = {}
        
        # Create entry for this II value
        entry = {
            'luts': design['luts'],
            'registers': design['registers'],
            'latency': design['latency'],
            'freq_mhz': design['freq_mhz']
        }
        
        # Add frames per second if requested
        if add_fps and design['latency'] is not None:
            fps = (design['freq_mhz'] * 1000000) / design['latency']
            entry['frames_per_sec'] = int(fps)
        elif add_fps:
            entry['frames_per_sec'] = None
        
        output[design_type][str(ii_value)] = entry
    
    return output

def main():
    parser = argparse.ArgumentParser(description='Generate JSON table from synthesis results and latency data')
    parser.add_argument('--results', required=True,
                        help='Path to results.json file with resource utilization data')
    parser.add_argument('--fil-lat', required=True,
                        help='Path to filament latencies JSON file')
    parser.add_argument('--rv-lat', required=True,
                        help='Path to RV latencies JSON file (pyramid-cycles.json)')
    parser.add_argument('--add-fps', action='store_true',
                        help='Add frames_per_sec field to the output')
    parser.add_argument('-o', '--output', 
                        help='Output file (default: stdout)')

    args = parser.parse_args()

    try:
        json_data = generate_json_table(args.results, args.fil_lat, args.rv_lat, args.add_fps)
        
        json_output = json.dumps(json_data, indent=2)
        
        if args.output:
            with open(args.output, 'w') as f:
                f.write(json_output)
            print(f"JSON output written to {args.output}", file=sys.stderr)
        else:
            print(json_output)
            
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
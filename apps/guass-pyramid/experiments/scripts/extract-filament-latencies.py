#!/usr/bin/env python3

import json
import sys
import subprocess
import os
import re
from pathlib import Path

def extract_latencies_from_folder(folder_path):
    """Extract latency and II data from blur_*.json files in a folder."""
    folder = Path(folder_path)
    if not folder.exists():
        print(f"Warning: Folder {folder_path} does not exist", file=sys.stderr)
        return []
    
    # Find all blur_*.json files in the folder
    json_files = list(folder.glob("blur_*.json"))
    if not json_files:
        print(f"Warning: No blur_*.json files found in {folder_path}", file=sys.stderr)
        return []
    
    results = []
    for json_file in json_files:
        try:
            # Run jq command to extract data
            cmd = [
                'jq', 
                '--arg', 'name', str(json_file),
                '{ name: $name, latency: .outputs[0].start, ii: .interfaces[0].delay }'
            ]
            
            with open(json_file, 'r') as f:
                result = subprocess.run(cmd, stdin=f, capture_output=True, text=True)
            
            if result.returncode == 0:
                data = json.loads(result.stdout.strip())
                results.append(data)
            else:
                print(f"Warning: Failed to process {json_file}: {result.stderr}", file=sys.stderr)
                
        except Exception as e:
            print(f"Warning: Error processing {json_file}: {e}", file=sys.stderr)
    
    return results

def clean_design_name(file_path):
    """Extract clean design name from file path."""
    # Extract the blur_X part from the file path
    match = re.search(r'blur_(\d+)\.json', file_path)
    if match:
        ii_value = match.group(1)
        return f"blur_{ii_value}"
    
    # Fallback to basename without extension
    return Path(file_path).stem

def transpose_data(data_list):
    """Transform from list of {name, latency, ii} to {latency: {name: value}, ii: {name: value}}."""
    result = {
        "latency": {},
        "ii": {}
    }
    
    for item in data_list:
        clean_name = clean_design_name(item["name"])
        result["latency"][clean_name] = item["latency"]
        result["ii"][clean_name] = item["ii"]
    
    return result

def main():
    if len(sys.argv) < 2:
        print("Usage: extract-filament-latencies.py <folder> [-o output_file]")
        print("Example: extract-filament-latencies.py fil-timing-adjusted-no-guards -o latencies.json")
        sys.exit(1)
    
    # Parse arguments
    folder = sys.argv[1]
    output_file = None
    
    if len(sys.argv) > 2 and sys.argv[2] == "-o" and len(sys.argv) > 3:
        output_file = sys.argv[3]
    
    # Extract data from folder
    print(f"Processing folder: {folder}", file=sys.stderr)
    data = extract_latencies_from_folder(folder)
    
    if not data:
        print("No data extracted from folder")
        sys.exit(1)
    
    # Transpose the data
    transposed = transpose_data(data)
    
    # Output to file or stdout
    if output_file:
        with open(output_file, 'w') as f:
            json.dump(transposed, f, indent=2)
        print(f"Extracted data for {len(data)} designs", file=sys.stderr)
        print(f"Output written to {output_file}", file=sys.stderr)
    else:
        print(json.dumps(transposed, indent=2))

if __name__ == '__main__':
    main()
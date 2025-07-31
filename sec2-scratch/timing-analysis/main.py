#!/usr/bin/env python3
"""
Main script for timing report processing.
Processes timing report files (.rpt) and converts them to structured JSON format.
"""

import argparse
import sys
from pathlib import Path

from timing import TimingReportParser


def process_single_file(input_file: str, output_file: str = None) -> bool:
    """Process a single timing report file."""
    try:
        print(f"Processing {input_file}...")

        # Parse the timing report
        parser = TimingReportParser(filepath=input_file)
        data = parser.parse()

        # Generate output filename if not provided
        if output_file is None:
            input_path = Path(input_file)
            output_file = str(input_path.with_suffix(".json"))

        # Write JSON output
        json_output = parser.to_json(output_file, indent=2)

        print(f"  ✓ Successfully converted to {output_file}")

        # Print summary
        paths_count = len(data.get("critical_paths", []))
        clocks_count = len(data.get("clocks", []))
        wns = data.get("timing_summary", {}).get("setup", {}).get("wns", 0.0)

        print(f"  ✓ Found {paths_count} critical paths, {clocks_count} clocks")
        print(f"  ✓ Setup WNS: {wns:.3f}ns")

        # Always print the JSON output
        print("\nJSON Output:")
        print(json_output)

        return True

    except Exception as e:
        print(f"  ✗ Error processing {input_file}: {str(e)}")
        return False


def process_directory(input_dir: str, output_dir: str = None) -> int:
    """Process all .rpt files in a directory."""
    input_path = Path(input_dir)

    if not input_path.exists():
        print(f"Error: Directory {input_dir} does not exist")
        return 1

    # Find all .rpt files recursively
    rpt_files = list(input_path.glob("**/*.rpt"))

    if not rpt_files:
        print(f"No .rpt files found in {input_dir}")
        return 0

    print(f"Found {len(rpt_files)} .rpt files in {input_dir}")

    # Setup output directory
    if output_dir:
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)
    else:
        output_path = input_path

    successful = 0
    failed = 0

    for rpt_file in rpt_files:
        # Calculate relative path for output
        rel_path = rpt_file.relative_to(input_path)
        output_file = output_path / rel_path.with_suffix(".json")

        # Create output directory if needed
        output_file.parent.mkdir(parents=True, exist_ok=True)

        if process_single_file(str(rpt_file), str(output_file)):
            successful += 1
        else:
            failed += 1

    print(f"\nProcessing complete: {successful} successful, {failed} failed")
    return 1 if failed > 0 else 0


def validate_schema(json_file: str, schema_file: str = None) -> bool:
    """Validate JSON output against schema (if available)."""
    try:
        import jsonschema
        import json

        if schema_file is None:
            schema_file = Path(__file__).parent / "schema.json"

        if not Path(schema_file).exists():
            print(f"Schema file {schema_file} not found, skipping validation")
            return True

        # Load schema and JSON
        with open(schema_file, "r") as f:
            schema = json.load(f)

        with open(json_file, "r") as f:
            data = json.load(f)

        # Validate
        jsonschema.validate(data, schema)
        print(f"  ✓ {json_file} validates against schema")
        return True

    except ImportError:
        print("  ⚠ jsonschema not installed, skipping validation")
        return True
    except Exception as e:
        print(f"  ✗ Schema validation failed: {str(e)}")
        return False


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Convert timing report files (.rpt) to structured JSON format",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s file.rpt                    # Convert single file to file.json
  %(prog)s file.rpt -o output.json     # Convert single file to output.json
  %(prog)s input_dir/                  # Convert all .rpt files in directory
  %(prog)s input_dir/ -o output_dir/   # Convert all files to output directory
  %(prog)s --test                      # Run test on example files
        """,
    )

    parser.add_argument(
        "input", nargs="?", help="Input .rpt file or directory containing .rpt files"
    )

    parser.add_argument(
        "-o",
        "--output",
        help="Output file or directory (default: same location as input with .json extension)",
    )

    parser.add_argument(
        "--validate", action="store_true", help="Validate output against JSON schema"
    )

    parser.add_argument("--schema", help="Path to JSON schema file for validation")

    parser.add_argument(
        "--test",
        action="store_true",
        help="Run tests on example files and synth-res directory",
    )

    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")

    args = parser.parse_args()

    if args.test:
        return run_tests()

    if not args.input:
        parser.print_help()
        return 1

    # Check if input is file or directory
    input_path = Path(args.input)

    if not input_path.exists():
        print(f"Error: {args.input} does not exist")
        return 1

    if input_path.is_file():
        success = process_single_file(args.input, args.output)

        if success and args.validate:
            output_file = args.output or str(input_path.with_suffix(".json"))
            validate_schema(output_file, args.schema)

        return 0 if success else 1

    elif input_path.is_dir():
        return process_directory(args.input, args.output)

    else:
        print(f"Error: {args.input} is neither a file nor a directory")
        return 1


def run_tests() -> int:
    """Run tests on example files and synth-res directory."""
    print("Running timing report parser tests...\n")

    base_dir = Path(__file__).parent

    # Test 1: Process example file if it exists
    example_rpt = base_dir / "example.rpt"
    if example_rpt.exists():
        print("Test 1: Processing example.rpt")
        process_single_file(str(example_rpt))

        # Compare with expected output if it exists
        example_json = base_dir / "example.json"
        output_json = base_dir / "example.json"

        if example_json.exists() and output_json.exists():
            print("  ✓ Comparing with expected output...")
            # Could add detailed comparison here

        print()  # Test 2: Process synth-res directory
    synth_res_dir = base_dir.parent / "synth-res"
    if synth_res_dir.exists():
        print("Test 2: Processing synth-res directory")
        process_directory(str(synth_res_dir))
        print()
    else:
        print("synth-res directory not found, skipping directory test")

    print("Tests completed!")
    return 0


if __name__ == "__main__":
    sys.exit(main())

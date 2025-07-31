# Timing Report Parser Tool

A comprehensive tool for extracting timing information from Vivado timing report files (.rpt) and converting them to structured JSON format for analysis.

## Features

- **Parse timing reports**: Extract structured data from Vivado timing reports
- **Batch processing**: Process multiple files or entire directories
- **JSON output**: Convert timing data to structured JSON format
- **Summary analysis**: Generate summary reports across multiple designs
- **Schema validation**: Validate output against JSON schema (if available)

## Files

- `timing.py` - Core timing report parser class
- `main.py` - Command-line interface for parsing reports
- `analyze.py` - Analysis tool for generating summary reports
- `schema.json` - JSON schema defining the expected output format
- `example.rpt` - Example timing report file
- `example.json` - Expected JSON output for the example

## Installation

No special installation required. The tool uses only Python standard library modules:
- `re` - Regular expressions for parsing
- `json` - JSON handling
- `datetime` - Date parsing
- `argparse` - Command-line parsing
- `pathlib` - Path handling
- `csv` - CSV report generation

## Usage

### Basic Usage

Convert a single timing report:
```bash
python main.py example.rpt
```

Convert with custom output file:
```bash
python main.py example.rpt -o output.json
```

Process entire directory:
```bash
python main.py ../synth-res/
```

### Run Tests

Test the parser on example files and process all reports:
```bash
python main.py --test
```

### Generate Summary Reports

Create a text summary of all timing results:
```bash
python analyze.py ../synth-res/
```

Generate CSV and JSON summaries:
```bash
python analyze.py ../synth-res/ --csv summary.csv --json summary.json
```

## Output Schema

The tool generates JSON files with the following structure:

```json
{
  "design_info": {
    "tool_version": "string",
    "date": "ISO date string",
    "name": "design name",
    "device": "FPGA device",
    "speed_grade": "string",
    "state": "design state"
  },
  "timing_summary": {
    "setup": {
      "wns": "float (ns)",
      "tns": "float (ns)",
      "failing_endpoints": "integer",
      "total_endpoints": "integer"
    },
    "hold": { /* same structure as setup */ },
    "pulse_width": { /* same structure as setup */ }
  },
  "clocks": [
    {
      "name": "clock name",
      "period": "float (ns)",
      "waveform": ["array of float"],
      "frequency_mhz": "float"
    }
  ],
  "critical_paths": [
    {
      "path_id": "integer",
      "status": "MET|VIOLATED",
      "slack": "float (ns)",
      "source": {
        "name": "endpoint name",
        "type": "input_port|output_port|register|combinational",
        "cell_type": "string (optional)",
        "clock": "string (optional)"
      },
      "destination": { /* same structure as source */ },
      "path_group": "string",
      "path_type": "string",
      "timing_requirement": "float (ns)",
      "data_path_delay": "float (ns)",
      "delay_breakdown": {
        "logic_delay": "float (ns)",
        "logic_delay_percentage": "float",
        "route_delay": "float (ns)",
        "route_delay_percentage": "float"
      },
      "logic_levels": {
        "total": "integer",
        "breakdown": { "cell_type": "count" }
      },
      "input_delay": "float (ns, optional)",
      "clock_path_skew": "float (ns, optional)",
      "clock_uncertainty": "float (ns, optional)",
      "arrival_time": "float (ns, optional)",
      "required_time": "float (ns, optional)",
      "data_path": ["array of timing steps"],
      "required_time_calculation": ["array of timing steps"]
    }
  ]
}
```

## Implementation Details

### TimingReportParser Class

The core parser class (`timing.py`) implements:

- **Section splitting**: Identifies major sections in timing reports
- **Design info extraction**: Parses tool version, date, device info
- **Timing summary parsing**: Extracts WNS, TNS, failing endpoints
- **Clock definitions**: Parses clock periods and waveforms
- **Critical path analysis**: Detailed path timing information
- **Robust parsing**: Handles various report formats and missing data

### Key Features

- **Error handling**: Graceful handling of malformed or missing data
- **Flexible input**: Accepts file paths or content strings
- **Extensible design**: Easy to add new parsing capabilities
- **Performance**: Efficient regex-based parsing

### Testing Results

The tool was tested on 84 timing report files from the `synth-res` directory:
- ✅ 84 files processed successfully
- ✅ 0 failures
- ✅ Correctly identified timing violations and critical paths
- ✅ Generated valid JSON output for all files

## Example Output Summary

From testing on the synthesis results:
- **6 timing summary reports** analyzed
- **3 designs** with setup violations (-1.710ns to -0.317ns)
- **6 designs** with hold violations (-1.663ns to -1.123ns)
- **All designs** had 20 critical paths each
- **Device**: xczu3eg FPGA across all designs

## Schema Validation

The tool supports JSON schema validation when the `jsonschema` package is available:

```bash
pip install jsonschema
python main.py example.rpt --validate
```

## Error Handling

The parser includes robust error handling for:
- Missing sections in timing reports
- Malformed timing data
- File I/O errors
- Regex parsing failures
- Invalid numeric values

Failed parses are reported but don't stop batch processing of multiple files.

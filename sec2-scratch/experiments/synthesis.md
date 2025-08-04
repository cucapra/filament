# Synthesizing Verilog Designs

## Synthesis Command
Use the `fud` tool to synthesize hardware designs using the command:
```bash
fud e --to synth-files --from synth-verilog -vv {VERILOG_FILE} \
 -s synth-verilog.tcl {SYNTH.tcl} \
 -s synth-verilog.constraints {CONSTRAINTS.xdc} \
  -o {OUT_DIR} 2> {LOG}.out
```

- The command takes a VERILOG FILE, a SYNTHESIS script, and a CONSTRAINTS file and produces all the resulting files into OUT_DIR. The {LOG}.out file will contain the information generated while synthesizing the design.

## Requirements
The command has the following constraints:
- The Verilog file must contain a top-level module called `main`
- The Verilog file may not depend on any other verilog files, i.e., all required
  modules must be contained within the file.

## TCL Script
- The TCL script should synthesize the design in out-of-context mode and run both `synth_1` and `impl_` steps.
- The example `synth/synth.tcl` file already does this so you should ask before generating another synthesis file

## Constraints File
- The constraints file `synth/device.xdc` defines a clock period of 7ns and connects the output and input ports to dummy pins.
- Ask before you attempt to generate a new constraints file and provide a rationale.

## Output Directory
- The output directory will contain the report files under the following structure: `{OUT_DIR}tmp.*/out/FutilBuild.runs/*rpt`

# Synthesizing and Checking a Design

- Synthesize the design in a new folder.
- Take Verilog design you can to synthesize and combine all of its dependent files into one file so that all the modules are available.
- Lint the file using Verilator and ensure there are no duplicate module definitions.
- First, sanity check that synthesis worked.
    - Check for the presence of {OUT_DIR}
    - Check that the {OUT_DIR} contains RPT files: `{OUT_DIR}/**/*.rpt | wc -l` should return a number greater than zero.
    - If any of these fail, look at the {LOG} file: `tail -20 {LOG}` for possible problems and document them.
- Next, use the `synthrep` tool to check that the design met timing
  - Run `synthrep -d {OUT_DIR} > summary.json` to collect information
  - Use `jq` to query the `meet_timing` and `worst_slack` fields. If the former is `0`, then the design failed to meet timing.
- Next, use the `synthrep` too to check the hierarchical resource usage:
  - Run `synthrep summary -m hierarchy -d {OUT_DIR}/new-static-1-1/tmp.*/out > hierarchy.json`
  - This file contains a summary from the hierarchy routed file generated during synthesis.
- Next, use the `../timing-analysis/parse.py` file on the timing file to extract the critical paths:
  - Run `../timing-analysis/parse.py {OUT_DIR}/**/main_timing_summary_routed.rpt > timing.json`
- Using the input verilog file and the information summary JSON files, characterize the design's timing behavior and resource usage.
- In your log book, create an entry for the design and summarize:
  - Critical paths by referencing specific Verilog code locations and snippets from the original file
  - Resource usage using the summary and hierarchy files.
  - Recommendations for how to improve the design's critical path or resource usage.

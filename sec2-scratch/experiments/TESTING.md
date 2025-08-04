# Testing Guide for Latency-Insensitive Interface Experiment

This document explains how to use Just (the `just` command) to execute test suites and individual testbenches for the floating point ALU experiment. This guide is designed for both human developers and Claude Code agents.

## Quick Start

```bash
# Run all tests (recommended for verification)
just test

# Get help with available commands  
just help

# List all available recipes
just --list

# Run a custom testbench
just run my_test.sv
```

## Test Suite Overview

The experiment includes three main test suites that verify functional correctness:

1. **Pipeline Variants Test** - Verifies mathematical equivalence across pipeline depths
2. **Static ALU Test** - Verifies latency-sensitive ALU with different configurations  
3. **Dynamic ALU Test** - Verifies latency-insensitive ALU with proper backpressure

## Individual Test Suites

### 1. Pipeline Variants Test (`test-variants`)

**Purpose**: Verifies that all pipeline variants (1-4 stages) of adders and multipliers produce identical results.

**Command**:
```bash
just test-variants
```

**What it verifies**:
- All 4 adder variants (1,2,3,4 stages) produce identical outputs for 1000 random test vectors
- All 4 multiplier variants (1,2,3,4 stages) produce identical outputs for 1000 random test vectors
- Mathematical correctness of pipeline distribution across stages
- No pipeline timing bugs or data path errors

**Test file**: `test_fp_variants.sv`

**Expected output**:
```
=== TEST RESULTS ===
Total tests: 1000
Adder errors: 0
Multiplier errors: 0
SUCCESS: All pipeline variants produce identical results!
```

### 2. Static ALU Test (`test-static`)

**Purpose**: Verifies the static (latency-sensitive) ALU implementation with different pipeline configurations.

**Command**:
```bash
just test-static
```

**What it verifies**:
- 4 different ALU configurations with varying adder/multiplier pipeline depths:
  - Config 0: 1-stage adder, 2-stage multiplier
  - Config 1: 2-stage adder, 3-stage multiplier  
  - Config 2: 3-stage adder, 1-stage multiplier
  - Config 3: 4-stage adder, 4-stage multiplier
- All configurations produce identical results for 100 test vectors
- Addition, subtraction, and multiplication operations work correctly
- Fixed pipeline latency behavior (results available after maximum pipeline depth)

**Test file**: `static/tests/test_static_alu.sv`

**Expected output**:
```
=== STATIC ALU TEST RESULTS ===
Total tests: 100
Errors: 0
SUCCESS: All pipeline configurations produce identical results!
```

### 3. Dynamic ALU Test (`test-dynamic-fixed`)

**Purpose**: Verifies the dynamic (latency-insensitive) ALU implementation with proper ready-valid handshaking.

**Command**:
```bash
just test-dynamic-fixed
```

**What it verifies**:
- 4 different LI ALU configurations with varying pipeline depths (same as static)
- Ready-valid handshaking protocol works correctly in both directions
- Backpressure propagation from output to input
- Valid bit persistence until transaction completion
- All configurations produce identical results for 100 test vectors
- Variable latency behavior (operations complete when ready, not on fixed schedule)

**Test file**: `dynamic/tests/test_dynamic_alu_fixed.sv`

**Expected output**:
```
=== FIXED DYNAMIC ALU TEST RESULTS ===
Total tests: 100
Results collected per ALU: 100 100 100 100
Errors found: 0
✅ SUCCESS: All dynamic ALU configurations produce identical results!
✅ Latency-insensitive interface working correctly with proper backpressure
```

## Running Custom Testbenches

### Generic Test Runner

Use the `run` recipe to compile and execute any custom Verilog testbench:

**Basic syntax**:
```bash
just run testbench_file.sv [sources] [output_name]
```

### Parameters

- **`testbench`** (required): Path to the testbench file
- **`sources`** (optional): Space-separated list of source files to include
  - Default: All project sources (`fp_*.sv` files)
- **`output_name`** (optional): Name for the compiled executable
  - Default: Testbench filename without `.sv` extension

### Examples

```bash
# Run testbench with default sources
just run my_debug_test.sv

# Run with specific source files
just run unit_test.sv "fp_adder_1stage.sv fp_mult_1stage.sv"

# Run with custom executable name
just run integration_test.sv "" integration

# Run testbench in subdirectory
just run debug/timing_test.sv

# Run with all project sources explicitly
just run full_test.sv "dynamic/main.sv static/main.sv fp_*.sv"
```

### Creating Custom Testbenches

When creating custom testbenches for this experiment:

1. **Include required modules**:
   ```systemverilog
   // Your testbench will have access to:
   // - FP_Adder_*Stage modules (1,2,3,4 stages)
   // - FP_Mult_*Stage modules (1,2,3,4 stages) 
   // - FP_Adder_Wrapper, FP_Mult_Wrapper (parameterized)
   // - FP_Adder_LI_Wrapper, FP_Mult_LI_Wrapper (latency-insensitive)
   // - Static_ALU (latency-sensitive ALU)
   // - Dynamic_ALU (latency-insensitive ALU)
   ```

2. **Use proper timescale**:
   ```systemverilog
   `timescale 1ns / 1ps
   ```

3. **For debugging ready-valid interfaces**:
   ```systemverilog
   // Monitor key signals
   $display("Cycle %d: ready_out=%b, valid_out=%b, result=%h", 
            cycle, ready_out, valid_out, result);
   ```

## Utility Recipes

### Linting
```bash
just lint
```
Runs Verilator lint on all source files to check for synthesis issues.

### Syntax Check
```bash
just syntax-check  
```
Quick syntax verification using iverilog without simulation.

### Clean
```bash
just clean
```
Removes all generated executables and temporary files.

### Build All Executables
```bash
just build-all
```
Pre-builds all test executables without running them.

### Debug Helpers
```bash
# Debug static ALU with default or custom testbench
just debug-static [testbench]

# Debug dynamic ALU with default or custom testbench  
just debug-dynamic [testbench]

# Generate VCD files for waveform analysis
just vcd testbench.sv [sources]
```

## Troubleshooting

### Common Issues

1. **"No such file or directory"**
   - Check that the testbench file exists and path is correct
   - Use `ls` to verify file presence before running

2. **"No top level modules"**
   - Ensure your testbench has a module with the same name as the filename
   - Check that all required source files are included

3. **Compilation errors**
   - Run `just lint` first to catch common issues
   - Check that all module dependencies are included in sources parameter

4. **Test failures**
   - For pipeline variants: Check for timing issues or incorrect pipeline stage distribution
   - For static ALU: Verify parameter passing and result multiplexing
   - For dynamic ALU: Check ready-valid protocol implementation and backpressure handling

### Debugging Tips

1. **Add debug output**:
   ```systemverilog
   $display("DEBUG: signal_name=%h at time %t", signal_name, $time);
   ```

2. **Generate VCD files** (add to testbench):
   ```systemverilog
   initial begin
       $dumpfile("debug.vcd");
       $dumpvars(0, testbench_module_name);
   end
   ```

3. **Use shorter test runs** for debugging:
   ```systemverilog
   parameter NUM_TESTS = 10;  // Reduce for faster debugging
   ```

## Integration with Claude Code

When using this testing framework with Claude Code agents:

1. **Always run full test suite first**: `just test`
2. **Use specific test recipes** for focused verification
3. **Create custom debug testbenches** using the `just run` recipe
4. **Check both functional correctness AND timing behavior** for LI interfaces
5. **Verify backpressure handling** explicitly in any custom LI implementations
6. **Use `just help` for detailed usage information**
7. **Use `just --list` to see all available recipes quickly**

## Expected Test Results

All tests should show **0 errors** for a functionally correct implementation:

- **Pipeline variants**: 1000 tests, 0 adder errors, 0 multiplier errors
- **Static ALU**: 100 tests, 0 errors across all configurations  
- **Dynamic ALU**: 100 tests, 100 results collected per ALU, 0 errors

Any non-zero error count indicates functional bugs that must be fixed before proceeding with performance analysis.
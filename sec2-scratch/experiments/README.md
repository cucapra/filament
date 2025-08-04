# Cost of Latency-Insensitive Interfaces Experiment

This experiment implements and compares static (latency-sensitive) vs dynamic (latency-insensitive) ALU implementations with varying pipeline depths for floating point addition and multiplication operations.

## Directory Structure

```
experiments/
├── README.md                    # This file
├── verilog/                     # Original implementations
│   ├── fp-add.sv               # 5-stage pipelined adder (reference)
│   └── fp-mult-nopipe.sv       # Combinational multiplier (reference)
├── fp_*_[1-4]stage.sv          # Pipeline variants (1-4 stages)
├── fp_*_wrapper.sv             # Parameterized wrappers
├── fp_*_li_wrapper.sv          # Latency-insensitive wrappers
├── test_fp_variants.sv         # Differential test for pipeline variants
├── static/                     # Static (latency-sensitive) ALU
│   ├── main.sv                 # Static ALU implementation
│   └── tests/
│       └── test_static_alu.sv  # Test bench
└── dynamic/                    # Dynamic (latency-insensitive) ALU
    ├── main.sv                 # Dynamic ALU implementation
    └── tests/
        └── test_dynamic_alu.sv # Test bench
```

## Implementation Overview

### Pipeline Variants

**Adder Variants (1-4 stages):**
- 1-stage: Input registration + combinational computation
- 2-stage: Input processing + alignment, then addition + normalization
- 3-stage: Input processing, mantissa alignment, addition + normalization
- 4-stage: Input processing, mantissa alignment, addition, normalization

**Multiplier Variants (1-4 stages):**
- 1-stage: Input registration + combinational computation
- 2-stage: Input processing + setup, then multiplication + result formation
- 3-stage: Input processing, multiplication, normalization + result formation
- 4-stage: Input processing, multiplication, normalization, rounding + result formation

### Static ALU

The static ALU (`static/main.sv`) implements a traditional latency-sensitive design:
- Fixed pipeline latency determined by the slowest functional unit
- All operations complete in the same number of cycles
- Simple control logic but potentially inefficient resource utilization

### Dynamic ALU  

The dynamic ALU (`dynamic/main.sv`) implements a latency-insensitive design:
- Ready-valid handshaking protocol
- Variable latency per operation type
- Internal buffering to handle backpressure
- More complex control but better resource utilization

## Key Design Decisions

### Pipeline Stage Distribution

**Adder Critical Path Analysis:**
- Exponent comparison and mantissa alignment (most critical)
- Mantissa addition/subtraction
- Result normalization and rounding

**Multiplier Critical Path Analysis:**
- Input preprocessing and hidden bit insertion
- 24x24 mantissa multiplication (most critical)
- Product normalization and rounding
- Exception handling

### Latency-Insensitive Interface

The LI wrappers implement:
- Input FIFOs to buffer incoming operands during backpressure
- Valid bit shift registers to track data through pipeline stages
- Handshaking logic to coordinate with upstream/downstream modules
- Stall detection and pipeline bubble handling

## Testing Strategy

### Differential Testing
All variants are tested to ensure they produce identical results:
1. **Pipeline Variants**: Test that 1, 2, 3, and 4-stage versions agree
2. **Static ALU**: Test different pipeline configuration combinations
3. **Dynamic ALU**: Test LI variants against each other and static versions

### Test Vectors
- Random floating point values
- Special cases (0.0, 1.0, -1.0, etc.)
- Edge cases for normalization
- Mixed operation types

## Performance Analysis

### Expected Characteristics

**Static ALU:**
- Throughput: 1 operation per MAX(adder_stages, mult_stages) cycles
- Latency: Fixed at maximum pipeline depth
- Area: Lower control overhead
- Power: May waste cycles on fast operations

**Dynamic ALU:**
- Throughput: Variable based on operation mix
- Latency: Operation-specific (better average case)
- Area: Higher due to buffering and control
- Power: Better utilization of functional units

### Bottleneck Analysis

For FPGA implementations, expected bottlenecks:
1. **Adder**: Carry propagation in mantissa addition
2. **Multiplier**: 24x24 multiplication (may use DSP blocks)
3. **Control Logic**: Ready-valid coordination overhead

## Usage

### Simulation
```bash
# Test pipeline variants
iverilog -o test_variants test_fp_variants.sv fp_*_[1-4]stage.sv
./test_variants

# Test static ALU
iverilog -o test_static static/tests/test_static_alu.sv static/main.sv fp_*.sv
./test_static

# Test dynamic ALU  
iverilog -o test_dynamic dynamic/tests/test_dynamic_alu.sv dynamic/main.sv fp_*.sv
./test_dynamic
```

### Synthesis
Use your preferred synthesis tool (Vivado, Quartus, etc.) to:
1. Synthesize different pipeline configurations
2. Compare area, timing, and power results
3. Analyze critical paths and resource utilization

## Results

The experiment allows comparison of:
- **Timing**: Critical path delay vs pipeline depth
- **Area**: Logic utilization and register overhead
- **Throughput**: Operations per second under different workloads
- **Latency**: Average and worst-case response times
- **Power**: Dynamic power under various operation mixes

## Future Extensions

1. **Variable Pipeline Depths**: Implement asymmetric pipeline configurations
2. **Multiple Functional Units**: Add parallel adders/multipliers  
3. **Advanced Scheduling**: Out-of-order execution for dynamic ALU
4. **Memory Interface**: Add load/store operations
5. **Precision Support**: Half, single, and double precision variants
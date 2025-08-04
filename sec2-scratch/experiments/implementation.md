# Cost of Latency-Insensitive Interfaces

**Goal**: Conduct an experiment on the costs of latency-insensitive experiments using an ALU with floating point adder and multiplier.

## Requirements
- You are given Verilog implementations of an adder and multiplier in the verilog/ directory. The implementation are assumed to be correct but do not do any pipelining.
- You may use `verilator` to lint the various verilog files and `iverilog` to simulate the test benches.
- Only the test benches may use unsynthesizable constructs.

## Implementing Variants

- Implement four variants of the adder and multiplier with 1, 2, 3, and 4 pipeline stages. Reason about the best locations to pipeline by thinking about FPGA designs and what parts are likely to be bottle necks.
- Build a test bench that differentially tests the results produced by the different implementations of adders and multipliers and make sure they agree with each other.

## Implementing Static ALU

- Next, we will implement a latency-sensitive ALU which can interface with the adder and multiplier variants.
- The first step is to build parameterized wrappers for the adder and the multiplier so that when the user sets the `STAGES` parameter, the implementation picks the appropriately pipelined adder and multiplier.
- Implement the ALU against these parameterized wrappers. The ALU should be paramterized by the number of stages in the adder and the multiplier. These arguments are passed to the underlying wrappers.
- Once the implementation is done, differentially test different combinations of parameters for the ALU and make sure they agree.

## Implement LI ALU

- Next, implement a latency-insensitve ALU which interfaces with the adder and multiplier variants using ready-valid signals.
- Again, the first step is to build wrappers that pick the correct variant based on the `STAGES` parameter and generate the correct ready and valid signals.
- Then, implement an ALU that is parameterized over the number of adder and multiplier stages and communicates using the ready-valid interface.
- Finally, differentially test the different variants of the dynamic ALUs against each other.

# Output

Generate the following directory format:

```
- static
  - main.sv   # implements the static ALU
  - tests/
- dynamic     # implements the LI ALU
  - main.sv
  - tests/
```

# Synthesis Experiment

Your job is to experiment various configurations of the static and dynamic ALU implementations to study to effects:
1. RQ1: How does changing the pipelining of the adder and multiplier modules change the resource usage and critical paths of the designs?
2. RQ2: How does using a ready-valid interface change the resource usage and maximum frequency of the resulting designs?


## Study
- Synthesizing designs is expensive. You will only get to synthesize 6 designs: 3 static configuration and 3 dynamic.
- Testing the designs using `verilator`, `synthrep`, and `../timing-analysis/parse.py` is cheap and you may choose to use it however you like.

## Prerequisites
- Read the `TESTING.md`, `synthrep.md`, and `synthesis.md` files to understand how the tools work with each other.
- Create a LOG.md to write down your thoughts as you run and debug experiments.
- Create a REPORT.md to track your user-facing report.

## Steps

- First, analyze what the critical paths of the static and dynamic ALUs look like. Read the add and muliply implementations and pick a combination of variants that are most likely to produce designs with high maximum frequency.
  - Use the same configuration to create top-level designs (as described in `synthesis.md`) and synthesize them. Using the same configuration will allow you to perform the comparisons needed for RQ2.
  - Perform the analysis described in `synthsis.md` and write down the observation in LOG.md
- Formulate a plan for the other four configurations you would like to study.
  - Write down your analysis for what the results from synthesizing the first two designs was and why the next designs would make sense.
  - Highlight how each of th designs would enable us to study RQ1 and RQ2 better.
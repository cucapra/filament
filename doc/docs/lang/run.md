# Running Filament Designs

Filament designs are compiled to Verilog using the [Calyx][] backend and simulated using tools like [Icarus Verilog][iverilog].
However, figuring out the right incantations to get these tools to work together and building testing harnesses can be tedious.
We use [fud][] to make the process of running Filament designs seamless: the user provides a file with the test data and runs a single command to compile, simulate, and generate outputs.

## Data Format

The test runner's data format is a JSON file that contains the names of each port mentioned in a Filament program's `main` component.
For example, for the [tutorial ALU][tut-alu] with the signature:
```filament
{{#include ../../examples/tut-wrong-1.fil:signature}}
```

We can have the following data file:
```json
{{#include ../../examples/data.json}}
```

The test harness operates with the idea of *transactions* where each transaction is a set of inputs and outputs corresponding to the indices into the JSON file.
For example, the first transaction sends the inputs `op[0]`, `left[0]`, and `right[0]` to the Verilog design and capture outputs for `out[0]`, corresponding to the output ports of the ALU.

This means, that the above data file will run the design with four inputs and capture four outputs. Adding another transaction is easy: just add another set of inputs to the JSON file.

## Running Designs

Running the design is straightforward, assuming you've [configured `fud`][fud-setup] already:
```sh
fud e --to cocotb-out examples/tut-seq.fil \
      -s cocotb.data examples/data.json \
      -s futil.flags ' -d canonicalize'
```

This instructs `fud` to compile the design to Verilog, setup the test harness, and run the simulation.
The output captures the values on the `out` port of the ALU for each transaction in the data file and tells us how many cycles it took to run the design:
```
{{#include ../../examples/run/tut-seq.expect:1}}
```

In general, the command to run designs is:
```sh
fud e --to cocotb-out <filament-file> \
      -s cocotb.data <data-file> \
      -s futil.flags ' -d canonicalize'
```

## Under the Hood

> Note: If you're following the tutorial, skip to the [Pipelining with Filament](./pipelining.md) section and come back here after you've finished.

Filament's test runner uses the signature of the `main` component to decide how long to provide inputs for, when to capture the outputs, and when to schedule the next transaction.

### Providing Inputs

The test harness holds the inputs for exactly the interval specified for each input and then provides [`'x` values][x-value] to the input.
This is done to make sure that incorrect designs that read inputs outside of their specified interval will not pass the test.

### Scheduling Transactions
Because Filament is represents hardware pipelines, new transactions can start before the previous transaction has finished.
By default, new transactions are scheduled exactly when the delay for the main event specifies.
For example, if the main event has a delay of `2`, then the next transaction will be scheduled after two cycles after starting the previous transaction.

However, it can be useful to change the scheduling behavior to check if there are pipelining bugs.
Our `fud`-based harness provides a way to randomize the timing of transactions by adding a random delay:
```sh
fud e --to cocotb-out examples/tut-seq.fil \
      -s cocotb.data examples/data.json \
      -s cocotb.randomize 10 \
      -s futil.flags ' -d canonicalize'
```

The `-s cocotb.randomize 10` flag adds a random delay of up to 10 cycles between transactions.

> Note: A well-typed Filament program produces the same output values regardless of the scheduling of transactions.

[calyx]: https://calyxir.org
[iverilog]: https://github.com/steveicarus/iverilog
[fud-setup]: ./start.html#full-build
[fud]: https://docs.calyxir.org/fud/index.html
[tut-alu]: ./lang/tutorial.html#building-an-arithmetic-logic-unit
[x-value]: https://stackoverflow.com/questions/69530556/what-exactly-do-x-and-z-values-represent-in-verilog
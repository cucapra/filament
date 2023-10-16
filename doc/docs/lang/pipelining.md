# Pipelining with Filament

While we've designed [a correct ALU][tut], it is quite slow: it processes one input completely before moving on to the next.
Such a hardware design is called a "fully sequential".
A standard technique to improve throughput of the design is [pipelining][], which allows a hardware module to process multiple inputs at the same time.

Filament is designed so that check that a pipeline can support the throughput specified in its interface.
We'll take our [sequential ALU][seq-alu] design and use Filament's type system to guide us to a correctly pipelined design.
Before that, however, let's run the design and see how it performs:
```bash
fud e --to cocotb-out examples/tut-seq.fil \
      -s cocotb.data examples/data.json \
      -s calyx.flags ' -d canonicalize'
```

Which generates the following output:
```
{{#include ../../../examples/run/tut-seq.expect:1}}
```

Note that this sequential design takes 12 cycles to process 4 inputs.

We'll start with pipelining our program:
```filament
{{#include ../../../examples/tut-seq.fil}}
```

## Delays and Throughput

Filament uses an event's *delay* to determine when the module is can accept new inputs.
```filament
{{#include ../../../examples/tut-seq.fil:sig}}
```
Note that the delay for the event `G` is `3` which indicates to Filament that the ALU process new inputs every three cycles.
We can tell Filament that we instead want a module that can process new inputs every cycle by changing the delay to `1`:
```filament
{{#include ../../../examples/tut-pipe-wrong-1.fil:sig}}
```

And run the compiler:
```
cargo run -- alu.fil
```

However, much to our dismay, Filament tells us that this program cannot be pipelined to achieve throughput 1.
However, being the nice type checker it is, it will tell us *exactly why* the design cannot be pipelined in the form of type errors.

Let's work through each error and see how we can fix it.

### Availability Intervals of Ports
The first error message points out that one of the inputs is required for three cycles, but the module may re-execute every cycle:
```
{{#include ../../../examples/tut-pipe-wrong-1.expect:4:13}}
```

This is problematic because `op` represents a *physical wire*; it is incapable of holding multiple values.
Our request to process inputs every cycle *and* have `op` last for three cycles is physically impossible.
The fix is easy: looking at our original design, we see that `op` is only used by the multiplexer in the interval [G+2, G+3) so we can change the availability interval of `op` to be [G+2, G+3).

> Note: If this step feel like divine insight, another way to reach the same conclusion is by changing the availability interval of `op` to be [G, G+1) which will cause the compiler to point out all availability intervals where `op` is used.

### Delays of Subcomponents
The second error message points out that the ALU component may execute every cycle but the multiplier we used can only execute every two cycles:
```
{{#include ../../../examples/tut-pipe-wrong-1.expect:13:25}}
```

Yet again, our request is physically impossible to satisfy: our multiplier circuit is fundamentally incapable of executing every cycle.
Thankfully for us, the `primitives/math.fil` file provides a component called `FastMult` which does have delay 1:
```filament
{{#include ../../../primitives/math/math.fil:fastmult}}
```

We can change out program to use this component instead:
```filament
{{#include ../../../examples/tut-pipe-wrong-2.fil}}
```

However, in making this change, we've created a new problem for ourselves:
```
{{#include ../../../examples/tut-pipe-wrong-2.expect:18:27}}
```
Filament tells us that `FastMult`'s `out` port is available in the interval [G+3, G+4) instead of [G+2, G+3) for `Mult`, i.e., the latency of `FastMult` is different from the latency of `Mult`.

Filament catching this bug is important-it would be very easy to miss such a mistake in a Verilog program.
Fixing it is quite mechanical:
```filament
{{#include ../../../examples/tut-pipe-wrong-3.fil}}
```

### Registers that Hold on for too Long

The final problem is quite similar to the previous one:
```
{{#include ../../../examples/tut-pipe-wrong-3.expect:4:18}}
```

The compiler is telling us the register's delay is 3 cycles.
However, unlike the multiplier, this is a consequence of our decision: we make the register hold on to its value for three cycles which increases its delay.
The last line of the error message points to the problem: the delay of a register *depends on how we use it*; this means that if we make it hold onto a value for exactly one cycle, its delay is reduced to one.

However, the problem is that we need the computation from the adder to be available three cycles from when it starts.
To get both the pipelining and correctness we want, we need to instantiate a *chain of registers* that feed values forward.

The intuition behind this is that because we want our ALU to process inputs every cycle, we need to "save" the computation in every cycle and push it forward.

The final program will look like this:
```filament
{{#include ../../../examples/tut-pipe.fil}}
```

## Running the Pipelined Design

Now to the moment of truth: let's run the design and see how it performs:
```bash
fud e --to cocotb-out examples/tut-pipe.fil \
      -s cocotb.data examples/data.json \
      -s calyx.flags ' -d canonicalize'
```

We get the following output which shows that the design took only 7 cycles to process 4 inputs:
```
{{#include ../../../examples/run/tut-pipe.expect:1}}
```

If you're still not convinced, try adding [another transaction][transaction] to the data file in `examples/data.json` and see how the cycle count for the original sequential and pipelined designs change.

## Something Remarkable

During the process of pipelining, we all of our time looking at type errors.
Once the program type checked, it produced the correct output *and* was correctly pipelined.
Filament supports many other features but at its heart, this is the guarantee it provides: if your program type checks, it is correctly pipelined.
Fast and correct, you can have both!

[tut]: ./tutorial.md
[pipelining]: https://cs.stanford.edu/people/eroberts/courses/soco/projects/risc/pipelining/index.html
[seq-alu]: ./tutorial.md#a-correct-implementation
[transaction]: ./run.md#data-format
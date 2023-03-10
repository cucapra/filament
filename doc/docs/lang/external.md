# Using Verilog Modules in Filament

Filament is designed to make it easy to use Verilog modules in a correct and efficient manner.
In short, to use a Verilog module, a Filament program needs to:
1. Provide the location of the Verilog module's source file
2. Provide a Filament signature for the Verilog module

## Using `extern` to Import Verilog Modules

Filament's `extern` keyword allows us to specify the signatures of all Verilog modules in a source file.
For example, if we have a file `modules.sv` that defines several modules:
```verilog
module Add(input [31:0] a, input [31:0] b, output [31:0] c);
...
endmodule

module Mult(input [31:0] a, input [31:0] b, output [31:0] c);
...
endmodule
```

We can use the `extern` block to specify the location of the file and provide Filament signatures for each module:
```filament
extern "modules.sv" {
comp Add<G: 1>(
    @[G, G+1] a: 32, @[G, G+1] b: 32
) -> (@[G, G+1] c: 32);

comp Mult<G: 1>(
    @[G, G+1] a: 32, @[G, G+1] b: 32
) -> (@[G+2, G+3] c: 32);
}
```

Note that unlike a Filament component, the `comp` definitions do not have a body; they simply define the signature of the Verilog module.

**Note.** The location of the Verilog file is determined relative to the location of the Filament file containing the `extern` block.

Once the definitions are specified, the Filament compiler will automatically link the Verilog modules into the final design.

## Defining the Right Interface

The trick with using external modules in Filament requires us to define the "right" interface.
For example, one way of defining a combinational component is something that produces its output in the same cycle as its inputs.
The following Filament signature captures this property
```filament
extern "comb.sv" {
comp Add<G: 1>(
    @[G, G+1] left: 32, @[G, G+1] right: 32
) -> (@[G, G+1] sum: 32);
}
```

The signature states that the `Add` module accepts an input in the first cycle and immediately produces the output in the same cycle.

However, another way to define a combinational component is something that can produce an output as long as its input is available; this means that the adder can produce the output for five or ten cycles *as long as* the input is available for *the same number of cycles*.

To capture such a signature, which can hold a signal for a caller defined number of cycles, we can use multiple events:
```filament
comp Add<G: L-(G), L: 1>(
    @[G, L] left: 32,
    @[G, L] right: 32
) -> (
    @[G, L] sum: 32
) where L > G;
```

The above signal states that the invocation of an `Add` instance gets to decide how long the output is available for.
We require that the event `L` occurs after `G` to ensure that the intervals are well-formed.
Finally, we also require that the delay of `G` is affected by the length of the signals; if the output is held for 10 cycles, then the adder cannot be used for 10 cycles.

Using such an component is straightforward:
```filament
A := new Add;
a := A<G, G+10>(l, r)
```

### Default Binding for Events

In the above example, the signature for `Add` is more flexible that the original one.
However, specifying the common case, where we use the adder for exactly one cycle, is a bit cumbersome:
```filament
A := new Add
a := A<G, G+1>(l, r)
```

Instead, we can provide a default binding for the event `L` that is used when the caller does not specify a value for `L`:
```filament
comp Add<G: L-(G), ?L=G+1: 1>(
    @[G, L] left: 32,
    @[G, L] right: 32
) -> (
    @[G, L] sum: 32
) where L > G;
```

The syntax `?L=G+1` tells the compiler to use the binding `G+1` for `L` when there is no binding provided for it.

> **Note.** Events with default bindings must occur after non-default events.

## Optimizing Verilog Modules using Filament Signatures

Filament's signatures are a powerful tool–if we know that a Verilog module is only going to be used in a certain way, we can optimize the module to be used in that way.
For example, if the module's interface requires that an input signal be available for multiple cycles, we don't have to save that signal in a register.
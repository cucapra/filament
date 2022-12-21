# Filament

Filament is a type system for hardware design languages (HDLs).
Most typed HDLs, like [Chisel][] or [Bluespec][], focus on the types of *values* such as ports.
Filament focuses on giving time-based signatures to module *interfaces*.

## An Example

The interface of a multiply-add unit might look like the following in Verilog syntax:
```verilog
module MA(
  input logic [31:0] left,
  input logic [31:0] right,
  input logic [47:0] acc,
  output logic [47:0] out,
);
```

This interface tells us that the first two ports take 32-bit inputs and the accumulator and output are 48-bits.
However, the actual interface of such a unit is often *time-based*:
- The `left` and `right` inputs should be provided in the first cycle
- The `acc` input should be provided in the second cycle
- The `out` output is available in the third cycle

Even though at first glance, this interface seems to reflect internal implementation details, it is in fact crucial that this interface be time-offset in this manner.
By offsetting the arrival of the `acc` signal, a user of the module can connect the `out` signal from the first iteration to the `acc` signal of a second iteration of this module, allowing the user to implement a dot-product unit that can process one input every cycle.

The given Verilog interface does not provide any of this information.
The user of this module needs to manually track this information outside the module and ensure that the timing behavior is correctly maintained.
Worse still, if the timing interface of the module changes in any way, the user is not warned and will silently perform the wrong computation.

The goal of Filament is to provide a way to *specify* and *check* such interfaces.

## Building

### Prerequisites
- The interval checker requires the `z3` binary to be installed.
    - Most platforms can just use their package manager. For example, on mac, run `brew install z3`.
    - See [installation instructions][z3-install].
- The compiler uses [Calyx][] to generate verilog designs. Clone the calyx repository in the parent folder for this repository: `git clone https://github.com/cucapra/calyx.git ../calyx`

Requires [Rust][] and `cargo` (automatically installed with Rust). Run `cargo build` to build the compiler and `cargo run` to run the compiler.

## Testing

The tests use [cocotb][] for cosimulation:
* Cocotb requires access to the `libpython.so` shared library.
  * Install the `find_libpython` package: `pip install find_libpython`
  * Check that the `find_libpython` package can find the `libpython.so` shared library: `python -c "import find_libpython; print(find_libpython.find())"`
  * If the above command doesn't return anything, reinstall python with shared libraries enabled. Using [pyenv][], the following command should work: `env PYTHON_CONFIGURE_OPTS="--enable-shared" pyenv install 3.10`
* Install dependencies: `pip install cocotb pytest`

Examples and tests are in the `tests/` directory.
Testing is done using [runt][]. To install, run `cargo install runt`. To run
tests, run: `runt` from the root directory.

## Syntax Highlighting

Syntax highlighting for (neo)vim implemented in `tools/vim`. To install using
a plugin manager like `vim-plug`, add the following your `.vimrc`:

```
Plug '<absolute path to filament>/toos/vim'
```

[runt]: https://docs.rs/runt/latest/runt/index.html
[rust]: https://www.rust-lang.org/
[z3-install]: https://github.com/Z3Prover/z3
[chisel]: https://www.chisel-lang.org/
[bluespec]: https://bluespec.com/
[calyx]: https://calyxir.org

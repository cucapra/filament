# Getting Started

Filament's compiler is implemented in using the Rust programming language.

## Prerequisites

The following commands are sufficient to build the Filament compiler and have it generate Verilog:
- [Install Rust][install-rust] which will configure the `cargo` tool.
- **TK**: Install Calyx/pin Filament to use particular release of Calyx IR.
- Install [z3][z3-install].
  - On Mac OS, this can be done using `brew install z3`.
- Build the compiler by running: `cargo build` in the root of the folder.

In order to simulate Filament programs, we need a couple more tools:
- Install [`fud`][fud] which manages hardware tools and makes it easy to test Filament programs.
  - Clone the [Calyx repository][calyx-repo]: `git clone https://github.com/cucapra/calyx`
  - Install [flit][]: `python -m pip install flit`
  - Install [fud][]: `cd calyx/fud && flit install -s`
  - Check `fud` was installed: `fud check`. It will report some tools are missing. This is expected.
- Install [Icarus Verilog][iverilog-install] and [configure `fud` to use it][fud-icarus].
  - Running `fud check` again should report that `icarus-verilog` was installed correctly.
- Install [cocotb][]: `python -m pip install cocotb`.
  - Cocotb install can often fail. Check it was installed correctly by running `python -c "import cocotb; print(cocotb.__version__)"`. If this command fails, see [Debugging Cocotb Installation](#debugging-cocotb-installation).
- Register Filament's fud stages by running the command in the filament repository: `fud register -p fud/filament.py filament`
    - Run `fud check` to make sure that the filament stages are correctly installed.
- Install [`runt`][runt]: `cargo install runt`

Once all tools are installed, running the following command should print out the test report:
```
runt -j 1
```


## Debugging Cocotb Installation

Cocotb requires the python shared library `libpython.so`/`libpython.dylib` (Mac OS) to work correctly. A common reason for a cocotb installation not working is when this library is missing.

To check if cocotb is able to find the shared library install `find_libpython`: `python -m pip install find_libpython`.

Next, run the following:
```
python -c "import find_libpython; print(find_libpython.find())"
```

If the above command does not print out anything, that means that the python library was not found and the python installation needs to be rebuilt.

If you use [`pyenv`][pyenv], the following command will install a python version with the shared library:
```
env PYTHON_CONFIGURE_OPTS="--enable-shared" pyenv install 3.10
```

Rerun the command to check that `libpython` was found after installing a new python version.



[install-rust]: https://www.rust-lang.org/tools/install
[fud]: https://docs.calyxir.org/fud/index.html
[iverilog-install]: https://iverilog.fandom.com/wiki/Installation_Guide
[fud-icarus]: https://docs.calyxir.org/fud/index.html#icarus-verilog
[cocotb]: https://www.cocotb.org/
[z3-install]: https://github.com/Z3Prover/z3
[flit]: https://flit.pypa.io/en/stable/
[calyx-repo]: https://github.com/cucapra/calyx
[runt]: https://github.com/cucapra/runt
[pyenv]: https://github.com/pyenv/pyenv
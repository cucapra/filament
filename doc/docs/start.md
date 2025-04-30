# Getting Started

Filament is a programming language for Fearless Hardware Design. It aims to enable software programmers without much hardware background to start designing performant hardware. At its heart, Filament uses a type system to encode properties important for designing efficient hardware. This guide helps you install the various tools to make Filament work.

## Minimal Build

A basic build, which does not support our automatic simulation harness, can be installed pretty easily: First, clone [this repository][fil-repo]: `git clone https://github.com/cucapra/filament.git`

Next, we can install the dependencies for the Filament compiler:
- [Install Rust][install-rust] which will configure the `cargo` tool.
- Install one of the two SMT solvers
  - Install [z3][z3-install]:
    - On Mac OS: `brew install z3`.
    - On Ubuntu: `apt install z3`
  - Install [cvc5][cvc5-install].
- Build the compiler by running: `cargo build` in the root of the folder.

To check that the compiler works, run the following command:
```
cargo run -- tests/run/add.fil
```
Which should generate the Verilog implementing the original program.

## Full Build

We'll need to install some tools from the [Calyx compiler][calyx-repo].

### Calyx Docker Image

Calyx tools are provided using a docker image:
```
docker run -it ghcr.io/cucapra/calyx:0.4.0
```

If you're using the container, skip to [configuring Filament tools](#configuring-filament-tools).

### Installing from Source

First, we need to configure the Calyx compiler which acts as the backend for Filament.
- Clone the [Calyx repository][calyx-repo]:
  ```
  git clone https://github.com/cucapra/calyx.git --depth 1 --branch v0.3.0
  ```
- Build the Calyx compiler:
  ```
  cd calyx && cargo build
  ```

In order to simulate Filament programs, we need a couple more tools:
- Install [`fud`][fud] which manages hardware tools and makes it easy to test Filament programs.
  - Install [flit][]: `python3 -m pip install flit`
  - Install [fud][]: `cd calyx/fud && flit install -s`
  - Check `fud` was installed: `fud check`. It will report some tools are missing. This is expected.
- Install [Icarus Verilog][iverilog-install] and [configure `fud` to use it][fud-icarus].
  - Running `fud check` again should report that `icarus-verilog` was installed correctly.
- Install [`runt`][runt]: `cargo install runt`
- Install [`jq`][jq]
  - On Mac OS: `brew install jq`
  - On Ubuntu: `apt install jq`

### Configuring Filament Tools

In the Filament repository, do the following:

- Install [cocotb][]: `python3 -m pip install cocotb`.
  - Cocotb install can often fail. Check it was installed correctly by running `python3 -c "import cocotb; print(cocotb.__version__)"`. If this command fails, see [Debugging Cocotb Installation](#debugging-cocotb-installation).
- Register Filament's fud stages by running the command in the filament repository: `fud register -p fud/filament.py filament`
    - Run `fud check` to make sure that the filament stages are correctly installed.

For a sanity check, run `fud check`. It should report that `iverilog`, `jq`, `filament`, `futil`, `cocotb` are correctly installed.

Once all tools are installed, running the following command should print out the test report:
```
runt -j 1 -o fail -d
```

## Next Steps

Now that we have installed the Filament compiler and accompanying tools, we can start using Filament. Use the following links to learn more about Filament:

- Writing your [first Filament Program](./lang/tutorial.md).
- [How do I integrate black-box Verilog with Filament](./lang/external.md)?

## Debugging Cocotb Installation

Cocotb requires the python shared library `libpython.so`/`libpython.dylib` (Mac OS) to work correctly. A common reason for a cocotb installation not working is when this library is missing.

To check if cocotb is able to find the shared library install `find_libpython`: `python3 -m pip install find_libpython`.

Next, run the following:
```
python3 -c "import find_libpython; print(find_libpython.find())"
```

If the above command does not print out anything, that means that the python library was not found and the python installation needs to be rebuilt.

If you use [`pyenv`][pyenv], the following command will install a python version with the shared library:
```
env PYTHON_CONFIGURE_OPTS="--enable-shared" pyenv install 3.10
```

Rerun the command to check that `libpython` was found after installing a new python version.



[install-rust]: https://www.rust-lang.org/tools/install
[fud]: https://docs.calyxir.org/running-calyx/fud/index.html
[iverilog-install]: https://iverilog.fandom.com/wiki/Installation_Guide
[fud-icarus]: https://docs.calyxir.org/running-calyx/fud/index.html#icarus-verilog
[cocotb]: https://www.cocotb.org/
[z3-install]: https://github.com/Z3Prover/z3
[flit]: https://flit.pypa.io/en/stable/
[calyx-repo]: https://github.com/cucapra/calyx
[runt]: https://github.com/cucapra/runt
[pyenv]: https://github.com/pyenv/pyenv
[jq]: https://stedolan.github.io/jq/
[fil-repo]: https://github.com/cucapra/filament
[cvc5-install]: https://github.com/cvc5/cvc5

# Filament

*A hype-system for hardware pipelines*

## Building

### Prerequisites
- The interval checker requires the `z3` binary to be installed.
    - Most platforms can just use their package manager. For example, on mac, run `brew install z3`.
    - See [installation instructions][z3-install].

Requires [Rust][] and `cargo` (automatically installed with Rust). Run `cargo build` to build the compiler and `cargo run` to run the compiler.

## Testing

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

# Filament

*A hype-system for hardware pipelines*

## Building

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

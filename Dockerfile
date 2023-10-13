FROM ghcr.io/cucapra/calyx:0.4.0

WORKDIR /home
ADD . filament
# Build the compiler
WORKDIR /home/filament
RUN CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse cargo build --all

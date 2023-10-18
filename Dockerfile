FROM ghcr.io/cucapra/calyx:0.4.0

# Install apt dependencies
RUN apt-get update -y && \
    apt-get install -y z3

# Install CVC5
WORKDIR /home
ENV PATH=$PATH:/root/.local/bin
RUN wget https://github.com/cvc5/cvc5/releases/download/cvc5-1.0.6/cvc5-Linux --output-document cvc5 && \
  chmod +x cvc5 && \
  mv cvc5 /root/.local/bin

# Install z3
WORKDIR /home
RUN git clone --depth 1 --branch z3-4.12.2 https://github.com/Z3Prover/z3.git
WORKDIR /home/z3
RUN python3 scripts/mk_make.py --prefix=/root/.local/ && \
  cd build && \
  make && \
  make install

# Add filament
WORKDIR /home
ADD . filament
# Build the compiler
WORKDIR /home/filament
# Make rust use sparse registries (https://doc.rust-lang.org/nightly/cargo/reference/registries.html#registry-protocols)
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo build --all
# Set up fud
RUN python3 -m pip install find-libpython && \
  fud register -p fud/filament.py filament

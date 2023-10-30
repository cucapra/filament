FROM ghcr.io/cucapra/calyx:0.4.0

# Install apt packages
RUN apt-get update -y && \
  apt-get -y install unzip

# Install CVC5
WORKDIR /home
ENV PATH=$PATH:/root/.local/bin
RUN wget https://github.com/cvc5/cvc5/releases/download/cvc5-1.0.6/cvc5-Linux --output-document cvc5 && \
  chmod +x cvc5 && \
  mv cvc5 /root/.local/bin

# Install z3
WORKDIR /home
RUN mkdir z3
WORKDIR /home/z3
RUN wget https://github.com/Z3Prover/z3/releases/download/z3-4.12.2/z3-4.12.2-x64-glibc-2.31.zip --output-document z3.zip && \
  unzip z3.zip
ENV PATH=$PATH:/home/z3/z3-4.12.2-x64-glibc-2.31/bin/

# Add filament
WORKDIR /home
ADD . filament
# Build the compiler
WORKDIR /home/filament
# Make rust use sparse registries (https://doc.rust-lang.org/nightly/cargo/reference/registries.html#registry-protocols)
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo build --all
# Set up fud
RUN python3 -m pip install cocotb find_libpython pytest && \
  fud register -p fud/filament.py filament && \
  fud config stages.filament.exec target/debug/filament && \
  fud config stages.filament.library .
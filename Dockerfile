FROM ghcr.io/cucapra/calyx:0.4.0

# Install apt dependencies
RUN apt-get update -y && \
    apt-get install -y z3

# Install CVC5
WORKDIR /home
ENV PATH=$PATH:/root/.local/bin
RUN wget https://github.com/cvc5/cvc5/releases/download/cvc5-1.0.6/cvc5-Linux --output-document cvcsou5 && \
  chmod +x cvc5 && \
  mv cvc5 /root/.local/bin

WORKDIR /home
ADD . filament
# Build the compiler
WORKDIR /home/filament
RUN CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse cargo build --all

FROM ghcr.io/cucapra/calyx:0.4.0

LABEL org.opencontainers.image.source https://github.com/cucapra/filament

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

# ----------------------------------------
# Install custom tools
# ----------------------------------------
WORKDIR /home

# Install required librariesg
RUN apt update && \
    DEBIAN_FRONTEND=noninteractive apt install -y autoconf automake \
    autotools-dev bison f2c flex git gpg g++ libblas-dev libboost-all-dev \
    liblapack-dev liblpsolve55-dev libsollya-dev libtool lp-solve ninja-build \
    pkg-config sollya wget


# Install latest cmake from source
RUN wget https://github.com/Kitware/CMake/releases/download/v3.28.0-rc3/cmake-3.28.0-rc3.tar.gz && \
    tar -xvf cmake-3.28.0-rc3.tar.gz && \
    cd cmake-3.28.0-rc3 && ./bootstrap &&\
    make && make install

WORKDIR /home
# Install FloPoCo
RUN git clone https://gitlab.com/flopoco/flopoco &&\
    cd flopoco && git checkout f3d76595c01f84cee57ae67eee1ceb31a6fe93bc &&\
    mkdir build && cd build &&\
    cmake -GNinja .. && ninja &&\
    ln -s /home/flopoco/build/code/FloPoCoBin/flopoco /usr/bin/flopoco

# Install GHDL
RUN apt install -y ghdl

# ----------------------------------------
# Install filament
# ----------------------------------------
WORKDIR /home
ADD . filament
# Build the compiler
WORKDIR /home/filament
RUN cargo build --all
# Set up fud
RUN python3 -m pip install cocotb find_libpython pytest && \
  fud register -p fud/filament.py filament && \
  fud config stages.filament.exec target/debug/filament && \
  fud config stages.filament.library .
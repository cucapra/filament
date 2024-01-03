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

# Install required libraries
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

# Install FloPoCo 4.1
WORKDIR /home
RUN git clone --depth 1 --branch flopoco-4.1 https://gitlab.com/flopoco/flopoco &&\
    cd flopoco && git checkout f3d76595c01f84cee57ae67eee1ceb31a6fe93bc &&\
    mkdir build && cd build &&\
    cmake -GNinja .. && ninja &&\
    ln -s /home/flopoco/build/code/FloPoCoBin/flopoco /usr/bin/flopoco

# Install GHDL 3.0.0
WORKDIR /home
# Install deps
RUN apt install -y gnat llvm clang
RUN git clone --depth 1 --branch v3.0.0 https://github.com/ghdl/ghdl.git &&\
    cd ghdl &&\
    mkdir build && cd build &&\
    LDFLAGS='-ldl' ../configure --with-llvm-config --prefix=/usr &&\
    make && make install

ARG TARGETARCH

# Install GCC 11
RUN apt install -y libgmp3-dev libmpfr-dev libmpc-dev
ENV LD_LIBRARY_PATH=/usr/local/lib64:$LD_LIBRARY_PATH
WORKDIR /home
RUN git clone --depth 1 --branch releases/gcc-11.4.0 https://github.com/gcc-mirror/gcc.git &&\
    cd gcc &&\
    ./configure --disable-multilib &&\
    make -j$(nproc) &&\
    make install

# Install libboost 1.84.0
WORKDIR /home
RUN git clone --depth 1 --recursive --branch boost-1.84.0 https://github.com/boostorg/boost.git &&\
    cd boost &&\
    ./bootstrap.sh &&\
    ./b2 install
# Add boost to include path
ENV CPLUS_INCLUDE_PATH=/home/boost:$CPLUS_INCLUDE_PATH

# Install SLANG
WORKDIR /home
RUN git clone --depth 1 --branch v5.0 https://github.com/MikePopoloski/slang.git &&\
    cd slang &&\
    cmake -B build &&\
    cmake --build build -j8 &&\
    cmake --install build --strip

# ----------------------------------------
# Install filament
# ----------------------------------------
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
FROM ghcr.io/cucapra/filament:dev

LABEL org.opencontainers.image.source https://github.com/cucapra/filament

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

# Install FloPoCo
RUN git clone https://gitlab.com/flopoco/flopoco &&\
    cd flopoco && git checkout f3d76595c01f84cee57ae67eee1ceb31a6fe93bc &&\
    mkdir build && cd build &&\
    cmake -GNinja .. && ninja &&\
    ln -s /home/flopoco/build/code/FloPoCoBin/flopoco /usr/bin/flopoco

# Install GHDL
RUN apt install -y ghdl
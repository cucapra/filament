
WORKDIR /home
RUN git clone --depth 1 --branch v5.0 https://github.com/MikePopoloski/slang.git &&\
    cd slang &&\
    cmake -B build &&\
    cmake --build build -j8 &&\
    cmake --install build --strip
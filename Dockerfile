# Build ygopro binary
FROM debian as ygopro_builder
WORKDIR /usr/src
RUN apt update && \
    env DEBIAN_FRONTEND=noninteractive apt install -y wget p7zip-full git build-essential libevent-dev libsqlite3-dev liblua5.3-dev && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    wget -O premake.zip https://github.com/premake/premake-core/releases/download/v5.0.0-alpha14/premake-5.0.0-alpha14-src.zip && \
    7z x -y premake.zip && \
    mv premake-5.0.0-alpha14 premake && \
    cd premake/build/gmake.unix && \
    make -j$(nproc) && \
    mv /usr/src/premake/bin/release/premake5 /usr/bin/premake5
RUN git clone --branch=server --recursive --depth=1 https://code.mycard.moe/mycard/ygopro && \
    cd ygopro && \
    git submodule foreach git checkout master && \
    premake5 gmake && \
    cd build && \
    make config=release -j$(nproc) && \
    cd ../.. && \
    mkdir ygopro-release && \
    cp -r ygopro/bin/release/ygopro ygopro/script ygopro/lflist.conf ygopro/cards.cdb ygopro-release/ 

# Build srvpru binary
FROM clux/muslrust as srvpru_builder
WORKDIR /usr/src/app/srvpru
COPY Cargo.* ./
COPY scanner/Cargo.* scanner/
COPY src/main.rs src/
COPY scanner/src/lib.rs scanner/src/
RUN cargo fetch
COPY src src
COPY scanner scanner
RUN cargo build --release

FROM debian 
WORKDIR /usr/src/app/srvpru
EXPOSE 7911 7922 7933
ENV SRVPRU_CONFIG_PATH /usr/src/app/srvpru/config
ENV RUST_LOG srvpru=info
RUN apt update && \
    env DEBIAN_FRONTEND=noninteractive apt install -y libevent-dev libsqlite3-dev liblua5.3-dev 
COPY --from=ygopro_builder /usr/src/ygopro-release ygopro
COPY --from=srvpru_builder /usr/src/app/srvpru/target/x86_64-unknown-linux-musl/release/srvpru .
ADD config config
ENTRYPOINT ["./srvpru"]
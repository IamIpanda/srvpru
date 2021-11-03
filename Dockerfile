# Build srvpru binary
FROM clux/muslrust as builder
WORKDIR /usr/src/app/srvpru
COPY Cargo.* .
COPY scanner/Cargo.* scanner/
COPY src/main.rs src/
COPY scanner/src/lib.rs scanner/src/
RUN cargo fetch
COPY src src
COPY scanner scanner
RUN cargo build --release

FROM debian 
WORKDIR /usr/src/app/srvpru
RUN apt update && \
    env DEBIAN_FRONTEND=noninteractive apt install -y wget git build-essential libevent-dev libsqlite3-dev p7zip-full python3 python-is-python3 liblua5.3-dev && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*
ENV SRVPRU_CONFIG_PATH /usr/src/app/srvpru/config
ENV RUST_LOG srvpru=info
COPY --from=srvpro:latest /ygopro-server/ygopro ygopro
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/srvpru .
ADD config config
ENTRYPOINT ["./srvpru"]
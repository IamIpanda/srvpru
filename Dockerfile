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

# ygopro:server is your ygopro provider.
FROM ygopro:server
WORKDIR /usr/src/srvpru
EXPOSE 7911
ENV SRVPRU_CONFIG_PATH /usr/src/srvpru/config
ENV RUST_LOG srvpru=info
COPY --from=srvpru_builder /usr/src/app/srvpru/target/x86_64-unknown-linux-musl/release/srvpru .
COPY config config
ENTRYPOINT ["./srvpru"]
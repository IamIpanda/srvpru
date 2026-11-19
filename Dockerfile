FROM node:22-alpine AS portal

ARG PROXY
ARG WITH_API=true

WORKDIR /portal
COPY package.json package-lock.json ./
COPY srvpro-portal-sdk/package.json ./srvpro-portal-sdk/
COPY srvpro-portal/package.json ./srvpro-portal/
COPY srvpro-api/package.json ./srvpro-api/
RUN if [ "$WITH_API" = "true" ]; then \
      if [ -n "$PROXY" ]; then npm config set proxy "$PROXY" && npm config set https-proxy "$PROXY"; fi \
      && npm ci; \
    fi
COPY srvpro-portal-sdk ./srvpro-portal-sdk
COPY srvpro-portal ./srvpro-portal
COPY srvpro-api/package.json srvpro-api/build.mjs srvpro-api/tsconfig.json ./srvpro-api/
COPY srvpro-api/src ./srvpro-api/src
RUN if [ "$WITH_API" = "true" ]; then \
      npm run build --workspace srvpro-portal \
      && npm run build --workspace srvpro-pages \
      && mkdir -p out \
      && cp -r srvpro-portal/dist/. out/ \
      && cp -r srvpro-api/dist/. out/; \
    else \
      mkdir -p out; \
    fi

FROM messense/rust-musl-cross:x86_64-musl AS build

ARG PROXY
ARG WITH_API=true
ENV http_proxy=$PROXY \
    https_proxy=$PROXY \
    HTTP_PROXY=$PROXY \
    HTTPS_PROXY=$PROXY

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY srvpro ./srvpro
COPY srvpro-api ./srvpro-api
COPY srvpro-app ./srvpro-app

RUN if [ "$WITH_API" = "true" ]; then \
      FEATURES=api,zip,card,forge,official; \
    else \
      FEATURES=zip,card,forge,official; \
    fi \
    && cargo build --release --target x86_64-unknown-linux-musl -p srvpro-app --no-default-features --features "$FEATURES"

FROM alpine:latest AS compress
ARG PROXY
ENV http_proxy=$PROXY \
    https_proxy=$PROXY \
    HTTP_PROXY=$PROXY \
    HTTPS_PROXY=$PROXY
RUN apk add --no-cache upx
COPY --from=build /build/target/x86_64-unknown-linux-musl/release/srvpro-app /srvpro
RUN upx --best --lzma /srvpro

FROM alpine:latest
WORKDIR /srvpro
ENV RUST_MIN_STACK=16777216 \
    SRVPRO_CONFIG_PATH=/srvpro/config
COPY --from=compress /srvpro /srvpro
COPY --from=portal /portal/out ./portal
COPY srvpro/config ./config
EXPOSE 7911
EXPOSE 7922
ENTRYPOINT ["/srvpro/srvpro"]

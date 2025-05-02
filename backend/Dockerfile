######## Build
FROM rust:1.86-alpine AS build-env
RUN apk add --no-cache musl-dev
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cp /app/target/x86_64-unknown-linux-musl/release/rurl-shortener /app/rurl-shortener

######## Run
FROM scratch
COPY --from=build-env /app/rurl-shortener /
ENTRYPOINT ["/rurl-shortener"]

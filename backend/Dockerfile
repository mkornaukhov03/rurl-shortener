FROM rust:1.86-alpine AS build-env
WORKDIR /app
COPY ./src ./src
COPY Cargo.toml Cargo.lock ./

RUN apk add --no-cache musl-dev
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN ls target

FROM scratch
COPY --from=build-env /app/target/x86_64-unknown-linux-musl/release/rurl-shortener /
ENTRYPOINT ["/rurl-shortener"]
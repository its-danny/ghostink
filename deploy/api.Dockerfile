FROM rust:1.86.0 AS builder
WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

RUN cargo build --release --bin ghostink-api

FROM debian:bookworm-slim AS runner
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/ghostink-api ./ghostink-api

CMD ["./ghostink-api"]

FROM rust:1-slim-trixie AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR /app

COPY --from=builder /app/target/release/stignore-manager /stignore-manager
COPY html/ /app/html/
COPY assets/ /app/assets/

ENTRYPOINT ["/stignore-manager"]
CMD ["/app/config.toml"]

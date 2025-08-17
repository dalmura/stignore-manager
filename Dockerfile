FROM rust:1-slim-trixie AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR /app

COPY --from=builder /app/target/release/stignore-manager /stignore-manager

ENTRYPOINT ["/stignore-manager"]
CMD ["/app/config.toml"]

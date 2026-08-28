FROM rust:1-slim-trixie AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY lib/ ./lib/
COPY agent/ ./agent/
COPY manager/src/ ./manager/src/
COPY manager/Cargo.toml ./manager/Cargo.toml

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp /app/target/release/stignore-agent /app/stignore-agent && \
    cp /app/target/release/stignore-manager /app/stignore-manager


FROM gcr.io/distroless/cc-debian13 AS agent

WORKDIR /app

COPY --from=builder /app/stignore-agent /stignore-agent

ENTRYPOINT ["/stignore-agent"]
CMD ["/app/config.toml"]


FROM gcr.io/distroless/cc-debian13 AS manager

WORKDIR /app

COPY --from=builder /app/stignore-manager /stignore-manager
COPY manager/html/ /app/html/
COPY manager/assets/ /app/assets/

ENTRYPOINT ["/stignore-manager"]
CMD ["/app/config.toml"]

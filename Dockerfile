FROM rust:1-slim-trixie AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY lib/ ./lib/
COPY agent/ ./agent/
COPY manager/ ./manager/

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

RUN cargo build --release


FROM gcr.io/distroless/cc-debian12 AS agent

WORKDIR /app

COPY --from=builder /app/target/release/stignore-agent /stignore-agent

ENTRYPOINT ["/stignore-agent"]
CMD ["/app/config.toml"]


FROM gcr.io/distroless/cc-debian12 AS manager

WORKDIR /app

COPY --from=builder /app/target/release/stignore-manager /stignore-manager
COPY manager/html/ /app/html/
COPY manager/assets/ /app/assets/

ENTRYPOINT ["/stignore-manager"]
CMD ["/app/config.toml"]

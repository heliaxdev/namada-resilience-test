FROM lukemathwalker/cargo-chef:latest-rust-1.85 AS chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-1.85 AS cacher
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes \
    libprotobuf-dev \
    build-essential \
    clang-tools-16 \
    git \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    libudev-dev \
    && apt-get clean
WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-1.85 AS builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release --locked

FROM docker.io/debian:bookworm-slim
ARG TEST_SCRIPT="test_run.sh"
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes \
    ca-certificates \
    curl \
    build-essential \
    && apt-get clean && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/namada-chain-workload /app/namada-chain-workload 
COPY --from=builder /app/${TEST_SCRIPT} /app/run.sh

RUN chmod +x /app/run.sh

RUN curl -o /app/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params\?raw\=true \
    && curl -o /app/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true \
    && curl -o /app/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true

ENV NAMADA_MASP_PARAMS_DIR=/app

ENTRYPOINT ["/app/run.sh"]

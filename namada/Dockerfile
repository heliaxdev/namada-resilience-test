# Deprecated in favor of the instrumented version
# Following the instruction from https://docs.namada.net/introduction/install/source
# This is so we can do the localnet setup
FROM docker.io/rust:1.81-slim-bookworm AS builder
WORKDIR /namada

ARG GIT_SHA
RUN [ -z "$GIT_SHA" ] && echo "GIT_SHA is required" && exit 1 || true

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
    make \
    git-core \
    libssl-dev \
    pkg-config \
    libclang-13-dev \
    build-essential \
    protobuf-compiler \
    libprotobuf-dev \
    libudev-dev \
    curl \
    wget \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ADD --keep-git-dir=true https://github.com/namada-net/namada.git#$GIT_SHA .

# Be aware the patch could potentially be version dependent
# getting rid of the sudo
COPY ./scripts_get_cometbft.patch .
RUN git apply scripts_get_cometbft.patch

RUN make install

RUN mkdir -p /root/.masp-params \
    && curl -o /root/.masp-params/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params?raw=true \
    && curl -o /root/.masp-params/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true \
    && curl -o /root/.masp-params/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true

# Try to slim down the runner
FROM docker.io/rust:1.81-slim-bookworm

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
    ca-certificates \
    curl \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ENV NAMADA_MASP_PARAMS_DIR=/root/.masp-params

COPY --from=builder /root/.masp-params/* /root/.masp-params/
COPY --from=builder /namada/target/release/namada* /usr/local/bin/
COPY --from=builder /usr/local/bin/cometbft /usr/local/bin/cometbft
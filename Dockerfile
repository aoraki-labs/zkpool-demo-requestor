FROM --platform=$BUILDPLATFORM ubuntu:22.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    git \
    musl-dev \
    gcc \
    binutils \
    clang \
    golang \
    libssl-dev \
    pkg-config \
    libpq-dev \ 
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /target/src
COPY rust-toolchain .

COPY . .
RUN cargo build --release --target-dir /target && \
      mv /target/release/zkpool-demo-requestor / && rm -rf /target

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["/zkpool-demo-requestor"]
COPY --from=builder /zkpool-demo-requestor /

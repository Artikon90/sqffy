# Linux builder/runner for SQffy e2e tests.
FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Warm dependency cache with a dummy project copy of manifests first.
COPY Cargo.toml Cargo.lock ./
COPY locales ./locales
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY e2e ./e2e

RUN cargo build --tests --bins \
    && cargo test --no-run --tests \
    && rm -rf /app/target/tmp

CMD ["bash", "e2e/run.sh"]

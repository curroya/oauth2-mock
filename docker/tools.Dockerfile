FROM rust:1.99.0-slim-bookworm
RUN apt-get update \
  && apt-get install -y --no-install-recommends git ca-certificates \
  && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
RUN cargo install --locked cargo-nextest@0.9.146 cargo-deny@0.20.2 \
  && rm -rf /usr/local/cargo/registry

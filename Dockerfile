FROM --platform=$BUILDPLATFORM rust:1.99.0-slim-bookworm AS deps

ARG BUILDARCH
ARG TARGETARCH

WORKDIR /build
ENV CARGO_TERM_COLOR=never \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc

# Debian ships a cross gcc for the other architecture only; the native one is the image's gcc.
RUN case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu cross_packages='gcc-x86-64-linux-gnu libc6-dev-amd64-cross' ;; \
      arm64) triple=aarch64-unknown-linux-gnu cross_packages='gcc-aarch64-linux-gnu libc6-dev-arm64-cross' ;; \
      *) echo "no release build for '$TARGETARCH'" >&2 && exit 1 ;; \
    esac \
    && echo "$triple" > /build/target-triple \
    && if [ "$TARGETARCH" != "$BUILDARCH" ]; then \
         apt-get update \
         && apt-get install -y --no-install-recommends $cross_packages \
         && rm -rf /var/lib/apt/lists/*; \
       fi

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

RUN rustup target add "$(cat target-triple)" \
    && mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked --target "$(cat target-triple)" \
    && rm -rf src

FROM deps AS builder

COPY src src

RUN find src -name '*.rs' -exec touch {} + \
    && cargo build --release --locked --target "$(cat target-triple)" \
    && cp "target/$(cat target-triple)/release/oauth2-mock" /build/oauth2-mock

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /build/oauth2-mock /oauth2-mock
EXPOSE 5556
ENTRYPOINT ["/oauth2-mock"]

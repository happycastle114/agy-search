ARG RUST_IMAGE=docker.io/library/rust:1.97.0-bookworm@sha256:8fa55b2f3ddf97471ab6a767bfa3f37e6bad0986ba823e75fea57e2a2a5c3073
ARG RUNTIME_IMAGE=docker.io/library/debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251

FROM ${RUST_IMAGE} AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
RUN cargo build --locked --profile dist --features server \
    --bin agy-search --bin agy-search-server

FROM ${RUNTIME_IMAGE} AS runtime
ARG TARGETARCH
ARG AGY_VERSION=1.2.7
ARG AGY_SHA256_AMD64=e410dd56d8c213ef12643d3ff5eaaab57a17e05bbf72e9415322f23879fc4a18
ARG AGY_SHA256_ARM64=8ddbb669158de1d1bc4c1fe5c130dca8f51da80d62569a54a4133f06768a723b

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        ca-certificates=20250419~deb12u1 \
        curl=7.88.1-10+deb12u15 \
    && case "${TARGETARCH}" in \
        amd64) asset="agy_cli_linux_x64.tar.gz"; sha256="${AGY_SHA256_AMD64}" ;; \
        arm64) asset="agy_cli_linux_arm64.tar.gz"; sha256="${AGY_SHA256_ARM64}" ;; \
        *) echo "unsupported target architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && curl --fail --location --proto '=https' --tlsv1.2 \
        --output /tmp/agy.tar.gz \
        "https://github.com/google-antigravity/antigravity-cli/releases/download/${AGY_VERSION}/${asset}" \
    && echo "${sha256}  /tmp/agy.tar.gz" | sha256sum --check --strict \
    && tar --extract --gzip --file /tmp/agy.tar.gz --directory /tmp antigravity \
    && install --mode 0755 /tmp/antigravity /usr/local/bin/agy \
    && rm /tmp/agy.tar.gz /tmp/antigravity \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 agy-search \
    && useradd --uid 10001 --gid 10001 --create-home \
        --home-dir /home/agy-search --shell /usr/sbin/nologin agy-search \
    && mkdir --parents /workspace /home/agy-search/.gemini/antigravity-cli \
    && chown --recursive 10001:10001 /workspace /home/agy-search

COPY --from=builder /build/target/dist/agy-search /usr/local/bin/agy-search
COPY --from=builder /build/target/dist/agy-search-server /usr/local/bin/agy-search-server

ENV HOME=/home/agy-search \
    AGY_SEARCH_AGY_PATH=/usr/local/bin/agy \
    AGY_SEARCH_LISTEN=0.0.0.0:18091
USER 10001:10001
WORKDIR /workspace
VOLUME ["/home/agy-search"]
EXPOSE 18091
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["curl", "--fail", "--silent", "--show-error", "http://127.0.0.1:18091/healthz"]
ENTRYPOINT ["agy-search-server"]
CMD ["http"]

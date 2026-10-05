# Self-hosted Amata API server.
#
#   docker build -t amata .
#   docker run -d --name amata -p 9260:9260 \
#     -e AMATA_API_TOKEN=change-me \
#     -v amata-data:/data amata
#
# Then: curl -H "Authorization: Bearer change-me" localhost:9260/api/health
#
# Builder Vienna: full rust toolchain for the static lcms2 + dav1d links.
# Keep the toolchain pinned so press-pipeline artefacts stay reproducible.
FROM rust:1.85-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang libclang-dev pkg-config dav1d libdav1d-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
# CARGO_INCREMENTAL=0 keeps disk use flat (matches upstream dev habit).
RUN CARGO_INCREMENTAL=0 cargo build --release --bin amata

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates dav1d \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 amata
COPY --from=builder /build/target/release/amata /usr/local/bin/amata
USER amata
WORKDIR /data
EXPOSE 9260
ENV AMATA_API_TOKEN=""
ENTRYPOINT ["/usr/local/bin/amata"]
CMD ["serve", "--host", "0.0.0.0", "--port", "9260"]

# syntax=docker/dockerfile:1
#
# cipherscan-rust (github.com/Kenbak/cipherscan-rust) ships no Dockerfile.
# build.rs runs tonic_build, so protoc is needed; rocksdb builds from source.
#
# The tree must first be pointed at this fork's zebra-chain — see
# README.md, "Porting the indexer". Not --locked: zebra's Cargo.lock is used as
# the seed (it carries pins for crates crates.io has since yanked) and cargo has
# to extend it with cipherscan's own dependencies.
ARG RUST_IMAGE=rust:1-bookworm

FROM ${RUST_IMAGE} AS build
WORKDIR /src
RUN apt-get update \
 && apt-get install -y --no-install-recommends protobuf-compiler clang libclang-dev \
 && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/cipherscan-indexer /usr/local/bin/cipherscan-indexer
ENTRYPOINT ["cipherscan-indexer"]
CMD ["live"]

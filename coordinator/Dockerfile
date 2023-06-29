FROM rust:1.67 as builder
WORKDIR /usr/src/kv_demo

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        libclang-dev

# cache dependencies
RUN cargo init --bin
COPY Cargo.toml ./
# COPY Cargo.lock ./
# RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
RUN cargo build --release
RUN rm ./src/*.rs ./target/release/deps/coordinator*

# build
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/coordinator /usr/local/bin/coordinator
EXPOSE 8000-8099
CMD ["coordinator"]

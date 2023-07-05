FROM rust:1.68 as chef
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN cargo install cargo-chef
WORKDIR app
RUN set -eux;

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --bin app

FROM debian:bullseye-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/app /usr/local/bin
EXPOSE 8000-8099
ENTRYPOINT ["/usr/local/bin/app"]

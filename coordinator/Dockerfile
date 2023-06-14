FROM rustlang/rust:nightly as builder
WORKDIR /usr/src/coordinator

# cache dependencies
RUN cargo init --bin
COPY Cargo.toml ./
# COPY Cargo.lock ./
# RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
RUN cargo +nightly build --release
RUN rm ./src/*.rs ./target/release/deps/coordinator*

# build
COPY . .
RUN cargo +nightly install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/coordinator /usr/local/bin/coordinator
EXPOSE 8000-8099
CMD ["coordinator"]

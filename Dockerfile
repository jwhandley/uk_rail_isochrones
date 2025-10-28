FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN --mount=type=bind,source=.git,target=/app/.git,ro \
    git submodule update --init --recursive
RUN cargo build --release --bin uk_rail_isochrones

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/uk_rail_isochrones /usr/local/bin
ENTRYPOINT ["/usr/local/bin/uk_rail_isochrones"]

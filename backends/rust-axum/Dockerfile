FROM rust:slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/url-shortener-rust /usr/local/bin/url-shortener-rust

EXPOSE 8001

CMD ["url-shortener-rust"]

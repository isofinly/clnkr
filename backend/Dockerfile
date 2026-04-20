# Stage 1: Build
FROM rust:latest AS builder
WORKDIR /app
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Stage 2: Runtime
FROM debian:trixie-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        openssl \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/transcriber-api /app/transcriber-api
EXPOSE 8080
ENTRYPOINT ["/app/transcriber-api"]

# Stage 1: Build
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt install -y openssl ca-certificates
COPY --from=builder /app/target/release/transcriber-api /app/transcriber-api
EXPOSE 8080
ENTRYPOINT ["/app/transcriber-api"]

FROM rust:1.86-alpine AS builder

# Install dependencies
RUN apk add --no-cache musl-dev openssl-dev
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY . .

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-musl

# Final stage
FROM alpine:latest
RUN apk add --no-cache ca-certificates

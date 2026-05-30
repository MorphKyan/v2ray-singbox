# Build stage
FROM rust:1.93.1-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies and cache them
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release

# Copy the real source code
COPY src ./src

# Rebuild the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

# Install ca-certificates to support fetching HTTPS subscription links
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and template
COPY --from=builder /app/target/release/v2ray-singbox /app/v2ray-singbox
COPY template.yaml /app/template.yaml

ENV PORT=3000
EXPOSE 3000

CMD ["/app/v2ray-singbox"]

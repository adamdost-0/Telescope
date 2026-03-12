FROM rust:1.83-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p telescope-hub

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/telescope-hub /usr/local/bin/
ENV PORT=3001
EXPOSE 3001
CMD ["telescope-hub"]

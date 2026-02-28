# Build stage
FROM rust:1.93 as builder

WORKDIR /app

# Copy cargo files
COPY Cargo.toml Cargo.lock example-config.yaml ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends tzdata ffmpeg ca-certificates libssl3 \
    && ln -snf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
    && echo "Asia/Shanghai" > /etc/timezone \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/matrix-wechat /usr/bin/matrix-wechat
COPY --from=builder /build/example-config.yaml /opt/matrix-wechat/example-config.yaml
COPY --from=builder /build/docker-run.sh /docker-run.sh

RUN chmod +x /docker-run.sh

VOLUME /data
WORKDIR /data

CMD ["/docker-run.sh"]

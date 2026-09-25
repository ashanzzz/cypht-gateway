FROM rust:1.88-bookworm AS builder
WORKDIR /src
COPY . .
ARG BUILD_TIME=unknown
ENV BUILD_TIME=${BUILD_TIME}
RUN cargo build --release -p gatewayd -p cyphtctl -p cypht-mcp

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home gateway \
    && mkdir -p /var/lib/cypht-gateway \
    && chown -R gateway:gateway /var/lib/cypht-gateway
COPY --from=builder /src/target/release/gatewayd /usr/local/bin/gatewayd
COPY --from=builder /src/target/release/cyphtctl /usr/local/bin/cyphtctl
COPY --from=builder /src/target/release/cypht-mcp /usr/local/bin/cypht-mcp
USER gateway
ENV GATEWAY_BIND=0.0.0.0:8080 \
    GATEWAY_DB_PATH=/var/lib/cypht-gateway/gateway.db
VOLUME ["/var/lib/cypht-gateway"]
EXPOSE 8080 8790
ENTRYPOINT ["/usr/local/bin/gatewayd"]

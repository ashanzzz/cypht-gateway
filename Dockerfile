FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
ARG BUILD_TIME=unknown
ENV BUILD_TIME=${BUILD_TIME}
RUN cargo build --release -p gatewayd -p cyphtctl

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home gateway
COPY --from=builder /src/target/release/gatewayd /usr/local/bin/gatewayd
COPY --from=builder /src/target/release/cyphtctl /usr/local/bin/cyphtctl
USER gateway
ENV GATEWAY_BIND=0.0.0.0:8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/gatewayd"]

FROM rust:1.41.0 as build

WORKDIR /workspace
ADD . .

ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN cargo build --release

FROM debian

RUN apt update && apt install -y openssl ca-certificates

WORKDIR /app
COPY --from=build /workspace/target/release/cors-proxy /app/cors-proxy

ENTRYPOINT ["/app/cors-proxy"]

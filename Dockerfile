FROM rust:1.50.0 as build

#RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /workspace
ADD . .

#RUN cargo build --target x86_64-unknown-linux-musl --release
RUN cargo build --release

FROM debian

RUN apt update && apt install -y openssl ca-certificates

WORKDIR /app
COPY --from=build /workspace/target/release/cors-proxy /app/cors-proxy

ENTRYPOINT ["/app/cors-proxy"]

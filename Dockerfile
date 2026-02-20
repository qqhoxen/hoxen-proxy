FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/hoxen-proxy /usr/local/bin/hoxen-proxy
ENTRYPOINT ["hoxen-proxy"]
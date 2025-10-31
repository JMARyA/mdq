FROM rust:trixie as builder

COPY . /app
WORKDIR /app

RUN cargo build --release

FROM debian:trixie

COPY --from=builder /app/target/release/mdq /mdq

ENTRYPOINT ["/mdq"]
CMD ["--help"]

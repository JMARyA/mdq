FROM rust:buster as builder

COPY . /app
WORKDIR /app

RUN cargo build --release

FROM ubuntu

COPY --from=builder /app/target/release/mdq /mdq

ENTRYPOINT ["/mdq"]
CMD ["--help"]
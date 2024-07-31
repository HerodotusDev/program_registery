FROM rust:1.67 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /usr/src/app/target/release/program_registery /usr/local/bin/program_registery
CMD ["program_registery"]
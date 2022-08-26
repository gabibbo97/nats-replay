# Build
FROM rust:1.63 AS build
COPY . /app
WORKDIR /app
RUN cargo build --release

# Run
FROM rust:1.63 AS app
COPY --from=build /app/target/release/nats-replay /nats-replay
ENTRYPOINT [ "/nats-replay" ]

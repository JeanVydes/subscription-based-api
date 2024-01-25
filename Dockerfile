# Dev (just for fast compilation)
# Build Stage
FROM rust:slim-buster AS builder

WORKDIR /usr/src/app

ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_TARGET_DIR=/usr/src/app/target

COPY Cargo.lock .
COPY Cargo.toml .
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev
RUN cargo build

# Runtime Stage
FROM fedora:34 AS runner

RUN dnf install -y libpq

EXPOSE 8080
COPY --from=builder /usr/src/app/target/debug/app /bin/app
ENTRYPOINT ["/bin/app"]
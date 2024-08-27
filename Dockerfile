FROM rust:1.80.0

WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release
ENV APP_ENVIRONMENT=prod
ENTRYPOINT ["./target/release/zero2prod"]
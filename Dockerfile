FROM rust:1 as builder

ARG SCHLOSS_VERSION
ENV SCHLOSS_VERSION = $SCHLOSS_VERSION

WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/schloss /
EXPOSE 8080
CMD ["./schloss"]

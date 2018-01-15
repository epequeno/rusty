FROM alpine:latest
RUN apk --no-cache add ca-certificates

WORKDIR /app

ADD ./target/x86_64-unknown-linux-musl/release/rusty /app

CMD ["/app/rusty"]

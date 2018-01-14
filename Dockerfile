FROM rust:latest

WORKDIR /rusty

COPY . .

RUN cargo install

CMD ["rusty"]
#!/bin/bash

docker_username="epequeno"
app_name="rusty"

# build statically linked binary
docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release

# build final container
docker build -t ${app_name} .

# push to docker hub
docker login -u ${docker_username}
docker tag ${app_name} ${docker_username}/${app_name}
docker push ${docker_username}/${app_name}
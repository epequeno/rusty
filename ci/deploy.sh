docker login -u epequeno -p $DOCKER_PASSWORD
docker run --rm -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release
docker build -t rusty .
docker tag rusty epequeno/rusty
sudo chmod -R ugo+w $(pwd)/target
docker push epequeno/rusty
cargo publish --token $CRATESIO_API_KEY
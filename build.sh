#!/bin/bash

CURRENT_ACCOUNT_NUM="$(aws sts get-caller-identity | jq -r '.Account')"

if [ "${CURRENT_ACCOUNT_NUM}" != "${SADEVS_APPS_ACCT_NUM}" ]; then
    echo "incorrect AWS account! expected ${SADEVS_APPS_ACCT_NUM} got ${CURRENT_ACCOUNT_NUM}"
    exit 1
fi

aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin "${SADEVS_APPS_ACCT_NUM}.dkr.ecr.us-east-1.amazonaws.com/rusty"
docker run --rm -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release
docker build -t rusty .
docker tag rusty:latest "${SADEVS_APPS_ACCT_NUM}.dkr.ecr.us-east-1.amazonaws.com/rusty:latest"
docker push "${SADEVS_APPS_ACCT_NUM}.dkr.ecr.us-east-1.amazonaws.com/rusty:latest"
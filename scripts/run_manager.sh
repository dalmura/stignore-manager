#!/usr/bin/env bash

pushd manager/

RUST_LOG=error cargo run --bin stignore-manager config.toml &
MANAGER=$!

kill_manager() {
    kill $MANAGER
    exit 0
}

trap 'kill_manager' SIGINT

echo 'Manager running, CTRL+C to quit'
while true; do
    sleep 10
done

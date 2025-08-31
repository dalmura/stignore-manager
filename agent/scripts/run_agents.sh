#!/usr/bin/env bash

RUST_LOG=error cargo run config-agent1.toml &
AGENT1=$!
RUST_LOG=error cargo run config-agent2.toml &
AGENT2=$!
RUST_LOG=debug cargo run config-agent3.toml &
AGENT3=$!

kill_agents() {
    kill $AGENT1
    kill $AGENT2
    kill $AGENT3
    exit 0
}

trap 'kill_agents' SIGINT

echo 'Agents running, CTRL+C to quit'
while true; do
    sleep 10
done

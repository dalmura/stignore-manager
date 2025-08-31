#!/usr/bin/env bash

if [ -e /tmp/media ]; then
    rm -rf /tmp/media
fi

mkdir /tmp/media

./scripts/create_agents.sh /tmp/media
./scripts/run_agents.sh

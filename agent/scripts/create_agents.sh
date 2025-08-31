#!/usr/bin/env bash

TARGET_DIR="${1}"

if [ ! -e "${TARGET_DIR}" ]; then
    echo "ERROR: Target directory '${TARGET_DIR}' doesn't exist?"
    exit 1
fi

THIS_DIR=$(dirname ${0})
DATA_SCRIPT="${THIS_DIR}/create_fake_data.sh"

if [ ! -e "${DATA_SCRIPT}" ]; then
    echo "ERROR: Expected script '${DATA_SCRIPT}' is missing?"
    exit 1
fi

# Create folders and data for all agents
mkdir "${TARGET_DIR}/agent1"
mkdir "${TARGET_DIR}/agent2"
mkdir "${TARGET_DIR}/agent3"

${DATA_SCRIPT} "${TARGET_DIR}/agent1"
${DATA_SCRIPT} "${TARGET_DIR}/agent2"
${DATA_SCRIPT} "${TARGET_DIR}/agent3"

# Delete some specific things for each agent

## agent1 is missing
# Movie B
rm -rf "${TARGET_DIR}/agent1/movies/Movie B (2024)"

# Show A (entirely)
rm -rf "${TARGET_DIR}/agent1/tv/Show A (1989)"

# Show B / Season 2 (entirely)
rm -rf "${TARGET_DIR}/agent1/tv/Show B (1994)/Season 2"

# Show C / Season 1 / E03
rm -rf "${TARGET_DIR}/agent1/tv/Show C (1999)/Season 1/S01E03 - Ep 3 (1999).mkv"


## agent2 is missing
# Movie C
rm -rf "${TARGET_DIR}/agent2/movies/Movie C (2025)"

# Show A / Season 1 (entirely)
rm -rf "${TARGET_DIR}/agent2/tv/Show A (1989)/Season 1"

# Show B / Season 3 / E01
rm -rf "${TARGET_DIR}/agent2/tv/Show B (1994)/Season 3/S03E01 - Ep 1 (1994).mkv"


## agent3 has everything


find "${TARGET_DIR}"

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
rm -rf "${TARGET_DIR}/agent1/movies/Movie B (2024)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie D (2020)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie F (2022)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie H (2018)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie K (2015)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie M (2013)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie P (1992)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie S (2007)"
rm -rf "${TARGET_DIR}/agent1/movies/Movie V (2014)"

# Show A (entirely)
rm -rf "${TARGET_DIR}/agent1/tv/Show A (1989)"

# Show B / Season 2 (entirely)
rm -rf "${TARGET_DIR}/agent1/tv/Show B (1994)/Season 2"

# Show C / Season 1 / E03
rm -rf "${TARGET_DIR}/agent1/tv/Show C (1999)/Season 1/S01E03 - Ep 3 (1999).mkv"


## agent2 is missing
rm -rf "${TARGET_DIR}/agent2/movies/Movie C (2025)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie E (2021)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie G (2019)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie J (2020)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie L (2014)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie O (2011)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie R (2001)"
rm -rf "${TARGET_DIR}/agent2/movies/Movie U (2012)"

# Show A / Season 1 (entirely)
rm -rf "${TARGET_DIR}/agent2/tv/Show A (1989)/Season 1"

# Show B / Season 3 / E01
rm -rf "${TARGET_DIR}/agent2/tv/Show B (1994)/Season 3/S03E01 - Ep 1 (1994).mkv"


## agent3 is missing
rm -rf "${TARGET_DIR}/agent3/movies/Movie A (2023)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie D (2020)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie E (2021)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie F (2022)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie I (2017)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie K (2015)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie N (2019)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie Q (1983)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie T (2006)"
rm -rf "${TARGET_DIR}/agent3/movies/Movie W (2003)"


find "${TARGET_DIR}"

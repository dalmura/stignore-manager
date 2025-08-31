#!/usr/bin/env bash

TARGET_DIR="${1}"

if [ ! -e "${TARGET_DIR}" ]; then
    echo "Target directory '${TARGET_DIR}' doesn't exist?"
    exit 1
fi

# Movies
mkdir -p "${TARGET_DIR}/movies/Movie A (2023)"
head -c 10M </dev/urandom >"${TARGET_DIR}/movies/Movie A (2023)/Movie A (2023).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie B (2024)"
head -c 20M </dev/urandom >"${TARGET_DIR}/movies/Movie B (2024)/Movie B (2024).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie C (2025)"
head -c 30M </dev/urandom >"${TARGET_DIR}/movies/Movie C (2025)/Movie C (2025).mkv"

touch "${TARGET_DIR}/movies/.stfolder"
mkdir "${TARGET_DIR}/movies/.stversions"

# TV Shows
mkdir -p "${TARGET_DIR}/tv/Show A (1989)/Season 1"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show A (1989)/Season 1/S01E01 - Ep 1 (1989).mkv"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show A (1989)/Season 1/S01E02 - Ep 2 (1989).mkv"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show A (1989)/Season 1/S01E03 - Ep 3 (1989).mkv"

mkdir -p "${TARGET_DIR}/tv/Show A (1989)/Season 2"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show A (1989)/Season 2/S02E01 - Ep 1 (1989).mkv"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show A (1989)/Season 2/S02E01 - Ep 2 (1989).mkv"

mkdir -p "${TARGET_DIR}/tv/Show B (1994)/Season 1"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show B (1994)/Season 1/S01E01 - Ep 1 (1994).mkv"

mkdir -p "${TARGET_DIR}/tv/Show B (1994)/Season 2"
head -c 5M </dev/urandom >"${TARGET_DIR}/tv/Show B (1994)/Season 2/S02E01 - Ep 1 (1994).mkv"
head -c 5M </dev/urandom >"${TARGET_DIR}/tv/Show B (1994)/Season 2/S02E02 - Ep 2 (1994).mkv"

mkdir -p "${TARGET_DIR}/tv/Show B (1994)/Season 3"
head -c 5M </dev/urandom >"${TARGET_DIR}/tv/Show B (1994)/Season 3/S03E01 - Ep 1 (1994).mkv"
head -c 5M </dev/urandom >"${TARGET_DIR}/tv/Show B (1994)/Season 3/S03E02 - Ep 2 (1994).mkv"

mkdir -p "${TARGET_DIR}/tv/Show C (1999)/Season 1"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show C (1999)/Season 1/S01E01 - Ep 1 (1999).mkv"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show C (1999)/Season 1/S01E02 - Ep 2 (1999).mkv"
head -c 10M </dev/urandom >"${TARGET_DIR}/tv/Show C (1999)/Season 1/S01E03 - Ep 3 (1999).mkv"

touch "${TARGET_DIR}/tv/.stfolder"
mkdir "${TARGET_DIR}/tv/.stversions"

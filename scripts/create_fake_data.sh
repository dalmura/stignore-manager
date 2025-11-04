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

mkdir -p "${TARGET_DIR}/movies/Movie D (2020)"
head -c 15M </dev/urandom >"${TARGET_DIR}/movies/Movie D (2020)/Movie D (2020).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie E (2021)"
head -c 18M </dev/urandom >"${TARGET_DIR}/movies/Movie E (2021)/Movie E (2021).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie F (2022)"
head -c 22M </dev/urandom >"${TARGET_DIR}/movies/Movie F (2022)/Movie F (2022).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie G (2019)"
head -c 25M </dev/urandom >"${TARGET_DIR}/movies/Movie G (2019)/Movie G (2019).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie H (2018)"
head -c 12M </dev/urandom >"${TARGET_DIR}/movies/Movie H (2018)/Movie H (2018).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie I (2017)"
head -c 16M </dev/urandom >"${TARGET_DIR}/movies/Movie I (2017)/Movie I (2017).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie J (2020)"
head -c 20M </dev/urandom >"${TARGET_DIR}/movies/Movie J (2020)/Movie J (2020).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie K (2015)"
head -c 14M </dev/urandom >"${TARGET_DIR}/movies/Movie K (2015)/Movie K (2015).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie L (2014)"
head -c 19M </dev/urandom >"${TARGET_DIR}/movies/Movie L (2014)/Movie L (2014).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie M (2013)"
head -c 23M </dev/urandom >"${TARGET_DIR}/movies/Movie M (2013)/Movie M (2013).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie N (2019)"
head -c 17M </dev/urandom >"${TARGET_DIR}/movies/Movie N (2019)/Movie N (2019).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie O (2011)"
head -c 21M </dev/urandom >"${TARGET_DIR}/movies/Movie O (2011)/Movie O (2011).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie P (1992)"
head -c 13M </dev/urandom >"${TARGET_DIR}/movies/Movie P (1992)/Movie P (1992).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie Q (1983)"
head -c 24M </dev/urandom >"${TARGET_DIR}/movies/Movie Q (1983)/Movie Q (1983).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie R (2001)"
head -c 11M </dev/urandom >"${TARGET_DIR}/movies/Movie R (2001)/Movie R (2001).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie S (2007)"
head -c 26M </dev/urandom >"${TARGET_DIR}/movies/Movie S (2007)/Movie S (2007).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie T (2006)"
head -c 28M </dev/urandom >"${TARGET_DIR}/movies/Movie T (2006)/Movie T (2006).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie U (2012)"
head -c 15M </dev/urandom >"${TARGET_DIR}/movies/Movie U (2012)/Movie U (2012).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie V (2014)"
head -c 27M </dev/urandom >"${TARGET_DIR}/movies/Movie V (2014)/Movie V (2014).mkv"

mkdir -p "${TARGET_DIR}/movies/Movie W (2003)"
head -c 29M </dev/urandom >"${TARGET_DIR}/movies/Movie W (2003)/Movie W (2003).mkv"

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

#!/bin/sh
set -eu

O="$(pwd)/packed/"
T=$(mktemp -p "$O" -u)

#probably not '-safe path passing here; can pee(1) even do it right?
H=$(pee sha256sum 'lz4 -5q - '\'"$T"\')
H=$(echo $H | cut -c1-64)

mv "${T}" "${O}${H}"
echo "${H}"

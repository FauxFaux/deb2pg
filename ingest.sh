#!/usr/bin/env bash
set -eu
set -o pipefail


for f in "$@"; do
    parts=$(echo $f | sed 's/_/ /; s/.orig.tar.*//')
    ~/code/contentin/target/release/ci-gen -h capnp $f \
     | ~faux/code/deb2pg/deb2pg-rs/target/release/deb2pg-rs ${parts}
done

#!/usr/bin/env bash
set -eu

for f in "$@"; do
    parts=$(echo $f | sed 's/_/ /; s/.orig.tar.*//')
    ~/code/contentin/target/release/ci-gen -h capnp $f \
     | ~faux/code/deb2pg/deb2pg-rs/target/release/deb2pg-rs ${parts}
     success=${PIPESTATUS[@]}
     if [ '0 0' != "${success}" ]; then
        echo $f >> ~/failures.log
     fi
done

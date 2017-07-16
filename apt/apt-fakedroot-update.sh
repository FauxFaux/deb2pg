#!/bin/bash
set -eu

mkdir -p fakedroot
cd fakedroot

mkdir -p etc/apt/apt.conf.d etc/apt/sources.list.d var/lib/apt/lists/partial var/lib/dpkg

if [ -f last-update ]; then
    if [ ! -n "$(find last-update -mmin +120)" ]; then
        exit 0
    fi
fi

touch var/lib/dpkg/status

cp -a --reflink=auto /usr/share/keyrings/debian-archive-keyring.gpg etc/apt/trusted.gpg

for dist in stable testing unstable experimental; do
    for prefix in 'deb' 'deb-src'; do
        echo ${prefix}' http://deb.debian.org/debian/ '${dist}' main'
    done
done > etc/apt/sources.list

printf '
Dir "'$(pwd)'";
Acquire::Pdiffs "false";
Debug::NoLocking "1";
APT::Architectures="amd64 i386";
' > etc/apt/apt.conf

APT_CONFIG=etc/apt/apt.conf apt-get update 1>&2

touch last-update


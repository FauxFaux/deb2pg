FROM debian:stretch

RUN apt-get install -y golang-1.8 ca-certificates git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash search
USER search

WORKDIR /home/search
RUN env PATH=/usr/lib/go-1.8/bin:$PATH go get github.com/junkblocker/codesearch/... \
    && rm -rf go/src go/pkg

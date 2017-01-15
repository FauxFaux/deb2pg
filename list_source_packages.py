#!/usr/bin/python3

import collections

import apt
import apt_pkg

cache = apt.cache.Cache(rootdir='fakedroot')

def versions_in(suite):
    source_versions = collections.defaultdict(set)

    for package in cache:
        for version in package.versions:
            if suite and suite not in (origin.archive for origin in version.origins):
                continue

            source_versions[version.source_name].add(version.source_version)

    return source_versions


if '__main__' == __name__:
    import sys
    sources = versions_in(sys.argv[1] if len(sys.argv) > 1 else None)
    for src in sorted(sources.keys()):
        # sort lexographically for determinism, not for any other reason
        for ver in sorted(sources[src]):
            print('{}={}'.format(src, ver))


#!/usr/bin/python3

import libarchive


def dump(entry: libarchive.ArchiveEntry):
    print(entry.filetype, entry.mode, entry.mtime, entry.name, entry.path, entry.pathname, entry.size, entry.linkname,
          entry.linkpath, entry.strmode)

def unpack(f: libarchive.read.ArchiveRead):
    for entry in f:
        dump(entry)


def main(path):
    with libarchive.file_reader(path) as f:
        unpack(f)


if __name__ == '__main__':
    import sys

    main(sys.argv[1])

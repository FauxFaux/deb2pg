#!/usr/bin/python3

import subprocess
import tarfile


# def dump(entry: libarchive.ArchiveEntry):
#     print(entry.filetype, entry.mode, entry.mtime, entry.name, entry.path, entry.pathname, entry.size, entry.linkname,
#           entry.linkpath, entry.strmode)


def unpack(readable):
    p = subprocess.Popen(['bsdtar', '-c', '-f', '-', '@-'], stdin=readable, stdout=subprocess.PIPE)
    with tarfile.open(mode='r|', fileobj=p.stdout) as tar:  # type: tarfile.TarFile
        for entry in tar:  # type: tarfile.TarInfo
            print(entry)
            if entry.isreg():
                unpack(tar.extractfile(entry))

    if 0 != p.wait():
        raise Exception('bsdtar failed')


def main(path):
    with open(path) as f:
        unpack(f)


if __name__ == '__main__':
    import sys

    main(sys.argv[1])

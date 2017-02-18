#!/usr/bin/python3
import io
import subprocess
import tarfile

# apt:python3-magic
import tempfile
from typing import List

import magic


# def dump(entry: libarchive.ArchiveEntry):
#     print(entry.filetype, entry.mode, entry.mtime, entry.name, entry.path, entry.pathname, entry.size, entry.linkname,
#           entry.linkpath, entry.strmode)
import shutil

UNPACKABLE_TYPES = {
    'application/x-tar',
}

M = magic.open(magic.MIME | magic.COMPRESS)
M.load()

ignored_mime_types = set()

def unpack(fd: io.BufferedReader, path: List[str]):
    print('unpacking {}'.format(path))
    p = subprocess.Popen(['bsdtar', '-c', '-f', '-', '@-'], stdin=fd, stdout=subprocess.PIPE, stderr=sys.stderr)
    with tarfile.open(mode='r|', fileobj=p.stdout) as tar:  # type: tarfile.TarFile
        for entry in tar:  # type: tarfile.TarInfo
            if not entry.isreg():
                if not entry.isdir():
                    print('irregular: {}//{}', path, entry)
                continue

            # returns different types if non-regular, but we know it's regular
            r = tar.extractfile(entry)  # type: tarfile.ExFileObject
            detected = M.buffer(r.peek(1024))
            mime_type, _ = detected.split('; ', 1)
            if mime_type not in UNPACKABLE_TYPES:
                ignored_mime_types.add(mime_type)
                continue

            with tempfile.TemporaryFile() as tmp:
                shutil.copyfileobj(r, tmp)
                tmp.flush()
                tmp.seek(0)
                unpack(tmp, path + [entry.name])

    if 0 != p.wait():
        raise Exception('bsdtar failed')


def main(path):
    with open(path) as f:
        unpack(f, [path])

    print(ignored_mime_types)


if __name__ == '__main__':
    import sys

    main(sys.argv[1])

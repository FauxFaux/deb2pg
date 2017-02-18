#!/usr/bin/python3
import io
import shutil
import subprocess
import tarfile
import tempfile
from typing import List

import magic

ARCHIVE_TYPES = {
    'application/java-archive',
    'application/x-archive',
    'application/x-tar',
    'application/zip',
}

MAGIC_COMPRESSED = magic.open(magic.MIME | magic.COMPRESS)
MAGIC_COMPRESSED.load()

MAGIC_MIME = magic.open(magic.MIME)
MAGIC_MIME.load()

ignored_mime_types = set()


def guess_can_extract(b: bytes) -> bool:
    """
    Guess if we want to try and run libarchive on this.
    Note that we don't want libarchive to try and process plain gzip files...
    they need handling differently.
    """

    if len(b) < 8:
        return False

    if b[0] in {
        b'#'[0],  # shebang, comment, #include, ...
        b'<'[0],  # html, <!doctype, <?php, xml, ...
    }:
        return False

    if b[0:2] in {
        b'/*',  # license header in c/java/...
    }:
        return False

    outer_mime = MAGIC_MIME.buffer(b)

    major, _ = outer_mime.split('/', 1)
    if major in {'text', 'image', 'audio', 'message'}:
        return False

    if outer_mime in ARCHIVE_TYPES:
        return True

    detected = MAGIC_COMPRESSED.buffer(b)
    mime_type, _ = detected.split('; ', 1)
    if mime_type in ARCHIVE_TYPES:
        return True

    ignored_mime_types.add(mime_type)
    return False


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
            if not guess_can_extract(r.peek(1024)):
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

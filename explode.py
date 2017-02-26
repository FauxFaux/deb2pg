#!/usr/bin/python3
import collections
import io
import shutil
import subprocess
import tarfile
import tempfile
from typing import List

import magic

ARCHIVE_TYPES = {
    'application/java-archive',
    # 'application/x-archive', (i.e. .lib files: not super useful,
    #  and fails anyway as the internals can't be repacked as .tar)
    'application/x-tar',
    'application/zip',
}

MAGIC_COMPRESSED = magic.open(magic.MIME | magic.COMPRESS)
MAGIC_COMPRESSED.load()

MAGIC_MIME = magic.open(magic.MIME)
MAGIC_MIME.load()

ignored_mime_types = set()
useless_miming = collections.Counter()


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
        b'\n'[0],  # probably text
    }:
        return False

    if b[0:2] in {
        b'/*',  # license header in c/java/...
        b'//',  # license header in c/java/...
        b'\xca\xfe',  # java .class
        b'\'\\',  # troff (e.g. man pages)
    }:
        return False

    if b[0:4] in {
        b'\x89PNG',
    }:
        return False

    if b[0:6] in {
        b'GIF87a',
        b'GIF89a',
        b'@echo ',  # bat files
        b'packag',  # java source
        b'import',  # java source
        b'Manife',  # java source
    }:
        return False

    outer_mime = MAGIC_MIME.buffer(b).split('; ')[0]

    major, _ = outer_mime.split('/', 1)
    if major in {'text', 'image', 'audio', 'message'}:
        useless_miming[b[0:6]] += 1
        return False

    if outer_mime in ARCHIVE_TYPES:
        print('outer: {}'.format(outer_mime))
        return True

    detected = MAGIC_COMPRESSED.buffer(b)
    mime_type, _ = detected.split('; ', 1)
    if mime_type in ARCHIVE_TYPES:
        print('inner: {}'.format(detected))
        return True

    if 'java-archive' in mime_type or '/zip' in mime_type:
        print("problemo")

    ignored_mime_types.add(mime_type)
    useless_miming[b[0:6]] += 1
    return False


def unpack(fd: io.TextIOWrapper, path: List[str]):
    print('unpacking {}'.format(path))
    p = subprocess.Popen(['bsdtar', '-c', '-f', '-', '@-'], stdin=fd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    with tarfile.open(mode='r|', fileobj=p.stdout) as tar:  # type: tarfile.TarFile
        for entry in tar:  # type: tarfile.TarInfo
            if not entry.isreg():
                if not entry.isdir():
                    print('irregular: {}//{}', path, entry)
                continue

            # returns different types if non-regular, but we know it's regular
            r = tar.extractfile(entry)  # type: tarfile.ExFileObject
            if not guess_can_extract(r.peek(64)):
                packer = subprocess.Popen(['./pack.sh'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                shutil.copyfileobj(r, packer.stdin)
                packer.stdin.close()
                if 0 != packer.wait():
                    raise Exception("packing failed")
                hash = packer.stdout.readline()
                continue

            with tempfile.TemporaryFile() as tmp:
                shutil.copyfileobj(r, tmp)
                tmp.flush()
                tmp.seek(0)
                unpack(tmp, path + [entry.name])

    if 0 != p.wait():
        print('bsdtar failed')


def main(path):
    with open(path) as f:
        unpack(f, [path])

    print(ignored_mime_types)
    print(useless_miming)


if __name__ == '__main__':
    import sys

    main(sys.argv[1])

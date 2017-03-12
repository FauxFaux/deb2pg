#!/usr/bin/python3
import collections
import hashlib
import io
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
from typing import Any, List, Tuple, Iterator

import magic

from deb2pg import BIN_DIR, TEXT_DIR, MANIFEST_DIR, ROOT_DIR, Entry

# for types only; actually using hashlib
try:
    from Crypto.Hash import SHA256
except ImportError:
    pass

ARCHIVE_TYPES = {
    'application/java-archive',
    # 'application/x-archive', (i.e. .lib files: not super useful,
    #  and fails anyway as the internals can't be repacked as .tar)
    'application/x-tar',
    'application/zip',
}


def info(msg: Any):
    sys.stderr.write("info: {}\n".format(msg))


def warn(msg: Any):
    sys.stderr.write("warn: {}\n".format(msg))


MAGIC_COMPRESSED = magic.open(magic.MIME | magic.COMPRESS)
MAGIC_COMPRESSED.load()

MAGIC_MIME = magic.open(magic.MIME)
MAGIC_MIME.load()

ignored_mime_types = set()
useless_miming = collections.Counter()


class ReNameableTemporaryFile:
    def __init__(self, create_in: str):
        self.name = tempfile.mktemp(dir=create_in, prefix='.', suffix='.tmp~')

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        try:
            os.unlink(self.name)
        except FileNotFoundError:
            pass


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
        info('outer: {}'.format(outer_mime))
        return True

    detected = MAGIC_COMPRESSED.buffer(b)
    mime_type, _ = detected.split('; ', 1)
    if mime_type in ARCHIVE_TYPES:
        info('inner: {}'.format(detected))
        return True

    ignored_mime_types.add(mime_type)
    useless_miming[b[0:6]] += 1
    return False


def handle_entry(entry: tarfile.TarInfo, tar: tarfile.TarFile, path: List[str]) -> Iterator[Entry]:
    our_path = path + [os.path.normpath(entry.name)]

    if not entry.isreg():
        if not entry.isdir():
            warn('irregular: {}//{}'.format(path, entry))
            yield Entry(our_path, entry.size, entry.mode, None, False)
        return

        # returns different types if non-regular, but we know it's regular
    r = tar.extractfile(entry)  # type: tarfile.ExFileObject

    if not guess_can_extract(r.peek(64)):
        digest, text = pack_into_temp_file(r)

        yield Entry(our_path, entry.size, entry.mode, digest, text)
        return

    with tempfile.TemporaryFile() as tmp:
        shutil.copyfileobj(r, tmp)
        tmp.flush()
        tmp.seek(0)
        yield from unpack(tmp, our_path)


def unpack(fd: io.BufferedReader, path: List[str]) -> Iterator[Entry]:
    info('unpacking {}'.format(path))
    p = subprocess.Popen(['bsdtar', '-c', '-f', '-', '@-'], stdin=fd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    try:
        with tarfile.open(mode='r|', fileobj=p.stdout) as tar:  # type: tarfile.TarFile
            for entry in tar:  # type: tarfile.TarInfo
                yield from handle_entry(entry, tar, path)
    except tarfile.ReadError as e:
        warn('bsdtar probably failed: {}'.format(e))

    if 0 != p.wait():
        warn('bsdtar failed')


def pack_into_temp_file(source: io.BufferedReader) -> Tuple[str, bool]:
    with ReNameableTemporaryFile(create_in=ROOT_DIR) as f:
        packer = subprocess.Popen(['lz4', '-5q', '-', f.name],
                                  stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE)
        h = make_hash()

        text = True
        while True:
            buf = source.read(16 * 1024)
            if not buf:
                break

            if text:
                text = is_text(buf)

            packer.stdin.write(buf)
            h.update(buf)

        packer.stdin.close()
        if 0 != packer.wait():
            warn(packer.stdout.read())
            warn(packer.stderr.read())
            raise Exception("packing failed")

        digest = h.hexdigest()
        dest = os.path.join(TEXT_DIR if text else BIN_DIR, digest)
        os.rename(f.name, dest)
        return digest, text


def make_hash() -> SHA256.SHA256Hash:
    return hashlib.sha256()


def is_text(buf: bytes) -> bool:
    """
    >>> is_text(b"\\0")
    False
    >>> is_text(b"\\t")
    True
    >>> is_text(b"\\r")
    True
    >>> is_text(b" ")
    True
    >>> is_text(b"Testing, testing,\\n123.")
    True
    """
    for char in buf:
        if char < 9 or 14 <= char < 32:
            return False
    return True


def main(path):
    with open(path, 'rb') as f, \
            ReNameableTemporaryFile(MANIFEST_DIR) as out, \
            open(out.name, 'w') as outf:

        h = hash_file(f)
        f.seek(0)
        json.dump({
            'name': os.path.basename(path),
            'hash': h,
        }, outf)
        outf.write('\n')

        for entry in unpack(f, []):
            json.dump(entry._asdict(), outf)
            outf.write('\n')

        os.rename(out.name, os.path.join(MANIFEST_DIR, h + ".manifest"))

    info(ignored_mime_types)
    info(useless_miming)


def hash_file(f: io.BufferedReader) -> str:
    h = make_hash()
    for chunk in iter(lambda: f.read(16 * 1024), b""):
        h.update(chunk)
    return h.hexdigest()


if __name__ == '__main__':
    import sys

    main(sys.argv[1])

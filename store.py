#!/usr/bin/python3

import collections
import math
import os
import queue
import subprocess
import threading
import time
from typing import Optional
from typing import Tuple

import psycopg2

INPUT_FROM = os.path.join(os.getcwd(), 'packed')
TEXT_DIR = os.path.join(INPUT_FROM, 'text')
BIN_DIR = os.path.join(INPUT_FROM, 'bin')

WorkItem = collections.namedtuple('WorkItem', ['is_text', 'hex_hash'])


def twos_complement(value: int, mask=2 ** 63) -> int:
    return -(value & mask) + (value & ~mask)


def decompose(hex_hash: str) -> Tuple[int, int, int, int]:
    """
    >>> decompose("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    (16406829232824261652, 11167788843400149284, 2859295262623109964, 11859553537011923029)
    """
    return tuple(twos_complement(int(hex_hash[i * 16: (i + 1) * 16], 16)) for i in range(4))


def write(is_text: bool, hex_hash: str, conn: psycopg2.extensions.connection):
    path = os.path.join(TEXT_DIR if is_text else BIN_DIR, hex_hash)
    size = os.path.getsize(path)
    try:
        h = decompose(hex_hash)
    except ValueError as _:
        # not a valid filename, ignore it
        return

    with conn.cursor() as curr:  # type: psycopg2.extensions.cursor

        curr.execute("""
INSERT INTO blob (len, h0, h1, h2, h3)
VALUES (%s, %s, %s, %s, %s)
ON CONFLICT DO NOTHING
RETURNING 1""", (size, *h))
        if curr.fetchone() is None:
            # we didn't insert the row, so no need to do anything
            # was hoping to get the position in the same statement,
            # but postgres doesn't work like that: returning only returns
            # modified rows
            os.unlink(path)
            return

        shard_no = make_shard_no(size)
        shard_name = '{}-{}'.format('text' if is_text else 'bin', shard_no)
        shard_id = (shard_no - 2)
        if not is_text:
            shard_id += 8

        try:
            pos = int(subprocess.check_output(['catfight', '-e', hex_hash, shard_name, path]).decode('utf-8'))
        except subprocess.CalledProcessError as e:
            import sys
            sys.stderr.write(e.output)
            raise

        pos += shard_id

        curr.execute("""
UPDATE blob SET pos=%s WHERE len=%s AND h0=%s AND h1=%s AND h2=%s AND h3=%s
""", (pos, size, *h))

    conn.commit()
    os.unlink(path)


def make_shard_no(size):
    """
    >>> make_shard_no(2000)
    3
    >>> make_shard_no(900)
    2
    >>> make_shard_no(5)
    2
    >>> make_shard_no(1e6)
    6
    >>> make_shard_no(9e99)
    9
    """
    return min(9, max(2, int(math.log10(size))))


class Worker(threading.Thread):
    def __init__(self, take_from: queue.Queue):
        super().__init__()
        self.take_from = take_from

    def run(self):
        with psycopg2.connect('') as conn:
            while True:
                struct = self.take_from.get()  # type: Optional[WorkItem]
                if not struct:
                    break
                write(struct.is_text, struct.hex_hash, conn)


def cores():
    import multiprocessing
    return multiprocessing.cpu_count()


def main():
    work = queue.Queue(maxsize=100)

    threads = cores() * 2

    for _ in range(threads):
        Worker(work).start()

    try:
        while True:
            for file in os.listdir(TEXT_DIR):
                work.put(WorkItem(True, file))
            for file in os.listdir(BIN_DIR):
                work.put(WorkItem(False, file))
            time.sleep(5)
    except KeyboardInterrupt as _:
        print('shutting down...')
        pass

    for _ in range(threads):
        work.put(None)


if '__main__' == __name__:
    main()

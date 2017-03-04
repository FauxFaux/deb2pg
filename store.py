#!/usr/bin/python3

import math
import os
import queue
import subprocess
import threading
from typing import Tuple

import psycopg2
import time

INPUT_FROM = os.path.join(os.getcwd(), 'packed')
TEXT_DIR = os.path.join(INPUT_FROM, 'text')
BIN_DIR = os.path.join(INPUT_FROM, 'bin')



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
    h = decompose(hex_hash)

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
            return

        shard = '{}-{}'.format('text' if is_text else 'bin', min(9, max(2, int(math.log10(size)))))
        try:
            pos = int(subprocess.check_output(['catfight', '-e', hex_hash, shard, path]).decode('utf-8'))
        except subprocess.CalledProcessError as e:
            import sys
            sys.stderr.write(e.output)
            raise

        curr.execute("""
UPDATE blob SET pos=%s WHERE len=%s AND h0=%s AND h1=%s AND h2=%s AND h3=%s
""", (pos, size, *h))

    conn.commit()
    os.unlink(path)


class Worker(threading.Thread):
    def __init__(self, take_from: queue.Queue):
        super().__init__()
        self.take_from = take_from

    def run(self):
        with psycopg2.connect('') as conn:
            while True:
                struct = self.take_from.get()
                write(struct[0], struct[1], conn)


def main():
    work = queue.Queue(maxsize=100)

    t = Worker(work)
    t.start()

    while True:
        for file in os.listdir(TEXT_DIR):
            work.put((True, file))
        for file in os.listdir(BIN_DIR):
            work.put((False, file))
        time.sleep(5)

if '__main__' == __name__:
    main()

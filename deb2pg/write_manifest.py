#!/usr/bin/python3
import json
import os
import sys
from typing import Dict, Iterator, Any, Tuple

import psycopg2

from deb2pg import Entry
from deb2pg.store import decompose


class StringPool:
    def __init__(self):
        self.cache = {}  # type: Dict[str, int]

    def get(self, item: str):
        ret = self.cache.get(item)
        if ret:
            return ret

        with psycopg2.connect('') as conn:
            with conn.cursor() as curr:  # type: psycopg2.extensions.cursor
                curr.execute("""
INSERT INTO path_component (path) VALUES (%s)
ON CONFLICT DO NOTHING
RETURNING id
""", (item,))
                inserted = curr.fetchone()
                if not inserted:
                    curr.execute("""
SELECT id FROM path_component WHERE path=%s
""", (item,))
                    inserted = curr.fetchone()

                self.cache[item] = inserted[0]
                return inserted[0]


def load(lines: Iterator[str]) -> Iterator[Entry]:
    for line in lines:
        yield Entry(**json.loads(line))


def write_package(curr: psycopg2.extensions.cursor, details: Dict[str, Any]) -> int:
    curr.execute("""
INSERT INTO container (info) VALUES (%s) RETURNING id""", (json.dumps(details),))
    return curr.fetchone()[0]


def find_pos(
        curr: psycopg2.extensions.cursor,
        decomposed: Tuple[int, int, int, int]):
    curr.execute("""
SELECT pos FROM blob WHERE h0=%s AND h1=%s AND h2=%s AND h3=%s""",
                 decomposed)
    return curr.fetchone()[0]


def main():
    sp = StringPool()
    manifest = sys.argv[1]
    with open(manifest) as f, \
            psycopg2.connect('') as conn, \
            conn.cursor() as curr:  # type: psycopg2.extensions.cursor

        lines = iter(f.readlines())
        pkgid = write_package(curr, json.loads(next(lines)))

        for entry in load(lines):
            path = [sp.get(item) for item in entry.name]
            pos = find_pos(curr, decompose(entry.hash))
            curr.execute("""
INSERT INTO file (container, pos, paths) VALUES (%s, %s, %s)""",
                         (pkgid, pos, path))
            # print(pkgid, path, entry.size, entry.mode, pos)
    os.unlink(manifest)


if '__main__' == __name__:
    main()

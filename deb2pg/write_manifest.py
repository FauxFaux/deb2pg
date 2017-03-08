#!/usr/bin/python3
import json
import sys
from typing import Dict, Iterator

import psycopg2

from deb2pg import Entry


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


def load(path: str) -> Iterator[Entry]:
    with open(path) as f:
        for line in f.readlines():
            yield Entry(**json.loads(line))


def main():
    sp = StringPool()
    for entry in load(sys.argv[1]):
        for item in entry.name:
            sp.get(item)


if '__main__' == __name__:
    main()

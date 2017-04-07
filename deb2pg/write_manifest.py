#!/usr/bin/python3
import collections
import json
import sys

import os
import psycopg2
from typing import Dict, Iterator, Any, Tuple, Iterable, Union, List, TypeVar

from deb2pg import Entry
from deb2pg.store import decompose

T = TypeVar('T')


class StringPool:
    def __init__(self):
        self.cache = {}  # type: Dict[str, int]
        self.get('')

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


def recursive_dict():
    return collections.defaultdict(recursive_dict)


def shortest_match(left: str, right: str):
    """
    >>> shortest_match("foo", "bar")
    ''
    >>> shortest_match("foo", "food")
    'foo'
    >>> shortest_match("abcd", "abfd")
    'ab'
    """

    shortest = min(len(left), len(right))
    i = 0

    while i < shortest:
        if left[i] != right[i]:
            break
        i += 1

    return left[0:i]


def find_prefix(within: Iterable[str]) -> str:
    """
    >>> find_prefix(["foo/bar", "foo/baz"])
    'foo/'
    >>> find_prefix([])
    ''
    >>> find_prefix(['one/two'])
    'one/'
    >>> find_prefix(['one/two', 'one'])
    ''
    >>> find_prefix(['one/two', 'two'])
    ''
    """
    it = iter(within)
    prefix = next(it, '')
    for item in it:
        if item.startswith(prefix):
            continue
        prefix = shortest_match(prefix, item)
    slash = prefix.rfind('/')
    if -1 == slash:
        return ''
    return prefix[0:slash + 1]


def strip_prefix(off_of: Dict[str, T]) -> Tuple[str, Dict[str, T]]:
    prefix = find_prefix(off_of.keys())
    if prefix:
        off_of = {k[len(prefix):]: v for k, v in off_of.items()}
    return prefix, off_of


def fixup_path_internal(
        structure: Dict[str, Union[Dict, int]],
        so_far: List[str]) -> Iterator[Tuple[List[str], int]]:
    """
    >>> list(fixup_path_internal({'a': {'b/c': 3, 'b/d': 4}}, []))
    [(['a', 'b/', 'c'], 3), (['a', 'b/', 'd'], 4)]
    """

    for item, sub in structure.items():
        if isinstance(sub, int):
            yield (so_far + [item], sub)
            continue

        prefix, sub = strip_prefix(sub)

        yield from fixup_path_internal(sub, so_far + [item, prefix])


def fixup_path(structure: Dict[str, Union[Dict, int]]) \
        -> Iterator[Tuple[List[str], int]]:
    """
    >>> list(fixup_path({'a/b': 3, 'a/c': 4}))
    [(['a/', 'b'], 3), (['a/', 'c'], 4)]
    """

    prefix, sub = strip_prefix(structure)
    yield from fixup_path_internal(sub, [prefix])


def main():
    sp = StringPool()
    manifest = sys.argv[1]
    with open(manifest) as f, \
            psycopg2.connect('') as conn, \
            conn.cursor() as curr:  # type: psycopg2.extensions.cursor

        lines = iter(f.readlines())
        pkgid = write_package(curr, json.loads(next(lines)))

        structure = recursive_dict()
        for entry in load(lines):
            ptr = structure
            last = entry.name.pop()
            for item in entry.name:
                ptr = ptr[item]
            ptr[last] = find_pos(curr, decompose(entry.hash))

        for row in fixup_path(structure):
            path = [sp.get(item) for item in row[0]]

            curr.execute("""
INSERT INTO file (container, pos, paths) VALUES (%s, %s, %s)""",
                         (pkgid, row[1], path))


if '__main__' == __name__:
    main()

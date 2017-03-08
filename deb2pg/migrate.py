#!/usr/bin/env python3
import os
import re
from typing import Tuple, Iterable

import psycopg2


def parse() -> Iterable[Tuple[str, Tuple[str, str]]]:
    root = 'migrations/'
    valid_name = re.compile('(\d{2})-(\w+).sql')
    for path in sorted(os.listdir(root)):
        ma = valid_name.match(path)
        if not ma:
            print('invalid file name: "{}"'.format(path))
            continue
        file_number = ma.group(1)
        file_name = ma.group(2)

        with open(root + path, 'r') as f:
            body = f.read()
        magic = '--migration'
        first_start = body.index(magic)
        body = body[first_start + len(magic):]
        for offset, part in enumerate(
                re.split('^' + magic + r'\b', body, flags=re.MULTILINE)):
            first_newline = part.index('\n')
            desc = part[:first_newline].strip()
            if not desc:
                desc = None
            part = part[first_newline:].strip()
            yield ('{}-{:02d}'.format(file_number, offset),
                   (part, '{}: {}'.format(file_name, desc) if desc else file_name))


def main():
    with psycopg2.connect('') as conn:
        with conn.cursor() as curs:  # type: psycopg2.extensions.cursor
            curs.execute('CREATE TABLE IF NOT EXISTS migrations('
                         'id VARCHAR NOT NULL PRIMARY KEY, '
                         'comment VARCHAR NOT NULL, '
                         'code VARCHAR NOT NULL)')
            curs.execute('SELECT id FROM migrations')
            done = set(x[0] for x in curs.fetchall())
            for id, operation in parse():
                if id in done:
                    continue
                curs.execute(operation[0])
                curs.execute('INSERT INTO migrations (id, comment, code) VALUES (%s, %s, %s)',
                             (id, operation[1], operation[0]))


if '__main__' == __name__:
    main()

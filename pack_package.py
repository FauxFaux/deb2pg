#!/usr/bin/env python3

import os
import subprocess
import tempfile

import psycopg2

import ingest_package


def eat(source_package, source_version):
    pack_dir = 'packs'

    with psycopg2.connect('dbname=deb2pg') as conn, \
            conn.cursor() as cur:
        # force the key into the table so we can lock it
        try:
            cur.execute('INSERT INTO packs VALUES (%s, NULL)', (source_package,))
            conn.commit()
        except psycopg2.InterfaceError:
            conn.rollback()

        cur.execute('select pack from packs for update where source_package=%s',
                    (source_package,))

        with tempfile.TemporaryDirectory() as tempdir:
            subprocess.check_call(['git', 'init'], cwd=tempdir)

            existing_pack = cur.fetchone()['pack']
            if existing_pack is not None:
                index_pack = subprocess.Popen(['git', 'index-pack', '--stdin'], cwd=tempdir)
                index_pack.communicate(input=existing_pack)

            tree = pack_package(tempdir, source_package, source_version)

        print(tree)


def ensure_exists(path):
    try:
        os.mkdir(path)
    except FileExistsError:
        pass


def pack_package(destdir, pack_dir, source_package, source_version):

    ingest_package.fetch(source_package, source_version, destdir)
    pkgfolder = os.path.join(destdir, 'pkg')
    subprocess.check_call(['git', 'add', '-A'], cwd=pkgfolder)
    tree = subprocess.check_output(['git', 'write-tree'], cwd=pkgfolder).decode('utf-8').strip()

    objects = subprocess.Popen(['git', 'rev-list', '--objects', '--format=%H', '--', tree],
                               cwd=pkgfolder, stdout=subprocess.PIPE)

    ensure_exists(tree[0:2])

    with open(tree_path(pack_dir, tree), 'wb') as f:
        subprocess.check_call(['git', 'pack-objects', '--stdout'],
                              stdin=objects.stdout,
                              stdout=f)

    if 0 != objects.wait():
        raise Exception('pipefail running rev-list')

    return tree


def tree_path(pack_dir, tree):
    return os.path.join(pack_dir, tree[0:2] + '/' + tree)


def main(specs):
    for spec in specs:
        # try:
        eat(*spec.split('=', 1))
        # except Exception as e:
        #     import traceback
        #     print(spec, ' failed to ingest', traceback.format_exc())


if __name__ == '__main__':
    import sys

    main(sys.argv[1:])

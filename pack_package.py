#!/usr/bin/env python3

import os
import subprocess
import tempfile

import ingest_package


def eat(source_package, source_version):
    pack_dir = 'packs'

    tree = pack_package(pack_dir, source_package, source_version)

    print(tree)


def ensure_exists(path):
    try:
        os.mkdir(path)
    except FileExistsError:
        pass


def pack_package(pack_dir, source_package, source_version):
    tmp_dir = 'tmp'

    ensure_exists(tmp_dir)

    with tempfile.TemporaryDirectory(dir=tmp_dir) as destdir:
        ingest_package.fetch(source_package, source_version, destdir)
        pkgfolder = os.path.join(destdir, 'pkg')
        subprocess.check_call(['git', 'init'], cwd=pkgfolder)
        subprocess.check_call(['git', 'add', '-A'], cwd=pkgfolder)
        tree = subprocess.check_output(['git', 'write-tree'], cwd=pkgfolder).decode('utf-8')

        objects = subprocess.Popen(['git', 'rev-list', '--objects', '--format=%H', '--', tree],
                                   cwd=pkgfolder, stdout=subprocess.PIPE)

        ensure_exists(tree[0:2])

        with open(os.path.join(pack_dir, tree[0:2] + '/' + tree), 'wb') as f:
            subprocess.check_call(['git', 'pack-objects', '--stdout'],
                                  stdin=objects.stdout,
                                  stdout=f)

        if 0 != objects.wait():
            raise Exception('pipefail running rev-list')

    return tree


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

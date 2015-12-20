#!/usr/bin/env python3

import os
import subprocess
import tempfile

import ingest_package


def eat(source_package, source_version):
    pack_dir = 'packs'

    tree = pack_package(pack_dir, source_package, source_version)

    print(tree)


def pack_package(pack_dir, source_package, source_version):
    try:
        os.mkdir('tmp')
    except FileExistsError:
        pass
    with tempfile.TemporaryDirectory(dir='tmp') as destdir:
        ingest_package.fetch(source_package, source_version, destdir)
        pkgfolder = os.path.join(destdir, 'pkg')
        subprocess.check_call(['git', 'init'], cwd=pkgfolder)
        subprocess.check_call(['git', 'add', '-A'], cwd=pkgfolder)
        tree = subprocess.check_output(['git', 'write-tree'], cwd=pkgfolder).decode('utf-8')

        # oh so ghetto; pack-objects is hard
        subprocess.check_call(['git', '--objects', '--format=%H', '--', tree], cwd=pkgfolder)
        pack_location = os.path.join(pkgfolder, '.git/objects/pack/')
        packs = [x for x in os.listdir(pack_location) if x.endswith('.pack')]
        if 1 != len(packs):
            raise ValueError("wrong number of packs", packs)
        pack = packs[0]

        os.renames(os.path.join(pack_location, pack), os.path.join(pack_dir, tree[0:2] + '/' + tree))
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

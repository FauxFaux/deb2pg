#!/usr/bin/env python3
import os
import re
import subprocess
import sys
import traceback

GEN = os.path.expanduser('~/code/contentin/target/release/ci-gen')
WRITE = os.path.expanduser('~/code/deb2pg/deb2pg-rs/target/release/deb2pg-rs')


def main():
    for path in sys.argv[1:]:
        try:
            filename = os.path.basename(path)
            name, crap = filename.split('_', 1)
            version = re.sub(r'\.orig\.tar\..*', '', crap)
            gen = subprocess.Popen([GEN, '-h', 'capnp', path], stdout=subprocess.PIPE)
            consume = subprocess.Popen([WRITE, name, version], stdin=gen.stdout)

            if 0 != gen.wait(60):
                raise Exception('gen failed')
            if 0 != consume.wait(60):
                raise Exception('consume failed')

        except Exception as e:
            traceback.print_exc()
            with open(os.path.expanduser('~/failure.log'), 'a') as f:
                f.write('{}\n'.format(path))


if __name__ == '__main__':
    main()

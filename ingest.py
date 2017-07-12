#!/usr/bin/env python3

# rm ingest.log; rm ~/failure.log; time find /mnt/data/sources -name \*.dsc -print0 | nice ionice xargs -0P8 -n20 python3 ingest.py 2>&1 | tee -a ingest.log

import itertools
import os
import re
import subprocess
import sys
import traceback
from email.message import Message
from email.parser import Parser

GEN = os.path.expanduser('~/code/contentin/target/release/ci-gen')
WRITE = os.path.expanduser('~/code/deb2pg/target/release/deb2pg-ingest')

FILES_LINE = re.compile(' [0-9a-f]{32} \d+ ([^ ]+)')


def main():
    for path in sys.argv[1:]:
        try:
            with open(path) as fp:
                dsc = deb822(fp)
            in_dir = os.path.dirname(path)
            name = dsc['Source']
            version = dsc['Version']
            files = [FILES_LINE.match(file).group(1) for file in dsc['Files'].split('\n')[1:]]
            print(name, version, files)

            gen = subprocess.Popen([GEN] + files, cwd=in_dir, stdout=subprocess.PIPE)
            consume = subprocess.Popen([WRITE, name, version], stdin=gen.stdout)

            if 0 != gen.wait(180):
                raise Exception('gen failed')
            if 0 != consume.wait(120):
                raise Exception('consume failed')

        except Exception as e:
            traceback.print_exc()
            with open(os.path.expanduser('~/failure.log'), 'a') as f:
                f.write('{}\n'.format(path))


# python3-debian's deb822 is GPL; bastards.
def deb822(fp) -> Message:
    # signed = subprocess.check_output(['gpg', '-v',
    #                                   '--keyring', '/usr/share/keyrings/debian-keyring.gpg'],
    #                                  stdin=fp, stderr=subprocess.DEVNULL)
    lines = list(itertools.dropwhile(lambda x: x.strip(), fp))
    msg = ''.join(lines[1:])
    return Parser().parsestr(msg, headersonly=True)


if __name__ == '__main__':
    main()

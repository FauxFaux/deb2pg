#!/usr/bin/env python3
import os
import sys


def ext(path):
    base, ext = os.path.splitext(path)
    return ext[1:] or base[base.rfind('/') + 1:]


def main():
    for line in sys.stdin.readlines():
        line = line.strip()
        print(ext(line))


if '__main__' == __name__:
    main()

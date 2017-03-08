import collections
import os

ROOT_DIR = os.path.join(os.getcwd(), 'packed')
TEXT_DIR = os.path.join(ROOT_DIR, 'text')
BIN_DIR = os.path.join(ROOT_DIR, 'bin')
MANIFEST_DIR = os.path.join(ROOT_DIR, 'manifests')

Entry = collections.namedtuple('Entry', ['name', 'size', 'mode', 'hash', 'text'])

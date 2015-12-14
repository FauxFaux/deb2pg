#!/usr/bin/env python3

import hashlib
import os
import subprocess
import tempfile

import apt
import psycopg2
from apt import apt_pkg

SIZE_LIMIT = 500 * 1024


def connect_to_db():
    return psycopg2.connect('dbname=deb2pg')


def fetch(source_name, source_version, destdir):
    src = apt_pkg.SourceRecords()
    acq = apt_pkg.Acquire(apt.progress.text.AcquireProgress())

    dsc = None
    source_lookup = src.lookup(source_name)

    # lifted directly from Package.fetch_source()
    while source_lookup and source_version != src.version:
        source_lookup = src.lookup(source_name)
    if not source_lookup:
        raise ValueError("No source for %s %s" % (source_name, source_version))
    files = list()
    for md5, size, path, type_ in src.files:
        base = os.path.basename(path)
        destfile = os.path.join(destdir, base)
        if type_ == 'dsc':
            dsc = destfile
        files.append(apt_pkg.AcquireFile(acq, src.index.archive_uri(path),
                                         md5, size, base, destfile=destfile))
    acq.run()

    for item in acq.items:
        if item.status != item.STAT_DONE:
            raise ValueError("The item %s could not be fetched: %s" %
                             (item.destfile, item.error_text))

    outdir = os.path.join(destdir, 'pkg')
    subprocess.check_call(["dpkg-source", "-x", dsc, outdir])
    return destdir


class BlobWriter:
    def __enter__(self):
        self.conn = connect_to_db()
        self.cur = self.conn.cursor()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        # don't care why; other people's problem
        self.cur.close()
        self.conn.close()

    def write_blob(self, blob):
        hasher = hashlib.sha1()
        hasher.update('blob {}\0'.format(len(blob)).encode('utf-8'))
        hasher.update(blob)
        hashed = hasher.hexdigest()
        try:
            blob = blob.decode('utf-8')
        except:
            pass

        uuid_part = hashed[0:32]

        # there's a race-condition here, but the unique index
        # will just crash us if we mess up anyway.

        upper_part = int(hashed[32:], 16)
        if upper_part & 0x80000000:
            upper_part -= 0x100000000

        for i in range(5):
            try:
                self.cur.execute('insert into blobs (hash_prefix, hash_suffix, content) select %s, %s, %s' +
                                 ' where not exists (select 1 from blobs where hash_prefix=%s)',
                                 (uuid_part, upper_part, blob, uuid_part))
                self.conn.commit()
                return uuid_part
            except psycopg2.IntegrityError:
                self.conn.rollback()
                pass


def eat(source_package, source_version):
    # root_dir('/home/faux/.local/share/lxc/sid/rootfs')

    with tempfile.TemporaryDirectory() as destdir, \
            connect_to_db() as conn, \
            conn.cursor() as cur, \
            BlobWriter() as writer:

        try:
            cur.execute('insert into packages(name, version, size_limit) values (%s, %s, %s) returning id',
                        (source_package, source_version, SIZE_LIMIT))
        except psycopg2.IntegrityError:
            # print(source_package, source_version, 'already exists, ignoring')
            return

        pkg_id = cur.fetchone()

        fetch(source_package, source_version, destdir)
        pkgfolder = os.path.join(destdir, 'pkg')
        for dirpath, _, filelist in os.walk(pkgfolder):
            for f in filelist:
                full_name = os.path.join(dirpath, f)
                stat = os.lstat(full_name)
                size = stat.st_size
                symlink = bool(stat.st_mode & 0o020000)
                user_exec = bool(stat.st_mode & 0o100)

                rel_path = os.path.join(os.path.relpath(dirpath, pkgfolder), f)
                if rel_path[0:2] == './':
                    rel_path = rel_path[2:]

                if symlink:
                    mode = '120000'
                elif user_exec:
                    mode = '100755'
                else:
                    mode = '100644'

                hashed = None
                if size < SIZE_LIMIT:
                    if not symlink:
                        with open(full_name, 'rb') as fh:
                            hashed = writer.write_blob(fh.read())
                    else:
                        hashed = writer.write_blob(os.readlink(full_name).encode('utf-8'))

                try:
                    cur.execute('insert into files (package, mode, path, hash_prefix) values (%s, %s, %s, %s)',
                                (pkg_id, mode, rel_path.encode('utf-8', 'backslashreplace').decode('utf-8'), hashed))
                except:
                    print('error processing ', source_package, source_version, hashed,
                          rel_path.encode('utf-8', 'backslashreplace'))
                    raise


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

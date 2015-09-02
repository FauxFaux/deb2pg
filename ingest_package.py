#!/usr/bin/env python3

import hashlib
import os
import subprocess
import tempfile

import apt
from apt import apt_pkg

import psycopg2

SIZE_LIMIT = 500 * 1024

def fetch(source_name, source_version, destdir):
    src = apt_pkg.SourceRecords()
    acq = apt_pkg.Acquire(apt.progress.text.AcquireProgress())

    dsc = None
    source_lookup = src.lookup(source_name)

    # lifted directly from Package.fetch_source()
    while source_lookup and source_version != src.version:
        source_lookup = src.lookup(source_name)
    if not source_lookup:
        raise ValueError("No source for %r" % self)
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
            raise FetchError("The item %r could not be fetched: %s" %
                             (item.destfile, item.error_text))


    outdir = os.path.join(destdir, 'pkg')
    subprocess.check_call(["dpkg-source", "-x", dsc, outdir])
    return destdir

# lifted directly from apt.cache.Cache():
def root_dir(rootdir):
    rootdir = os.path.abspath(rootdir)
    if os.path.exists(rootdir + "/etc/apt/apt.conf"):
        apt_pkg.read_config_file(apt_pkg.config,
                                    rootdir + "/etc/apt/apt.conf")
    if os.path.isdir(rootdir + "/etc/apt/apt.conf.d"):
        apt_pkg.read_config_dir(apt_pkg.config,
                                rootdir + "/etc/apt/apt.conf.d")
    apt_pkg.config.set("Dir", rootdir)
    apt_pkg.config.set("Dir::State::status",
                        rootdir + "/var/lib/dpkg/status")
    apt_pkg.config.set("Dir::bin::dpkg",
                        os.path.join(rootdir, "usr", "bin", "dpkg"))
    apt_pkg.init_system()

def ingest(pkg_id, fh, cur, size, name):
    hashed = None

    if size <= SIZE_LIMIT:
        blob = fh.read()
        hasher = hashlib.sha1()
        hasher.update('blob {}\0'.format(len(blob)).encode('utf-8'))
        hasher.update(blob)
        hashed = hasher.hexdigest()
        try:
            blob = blob.decode('utf-8')
        except:
            pass

        # there's a race-condition here, but the unique index
        # will just crash us if we mess up anyway.
        cur.execute('insert into blobs (hash, content) select %s, %s'
                            + ' where not exists (select 1 from blobs where hash=%s)',
                            (hashed, blob, hashed))

    cur.execute('insert into files (package, path, hash) values (%s, %s, %s)',
                (pkg_id, name, hashed))

def eat(source_package, source_version):
    #root_dir('/home/faux/.local/share/lxc/sid/rootfs')

    with tempfile.TemporaryDirectory() as destdir, \
            psycopg2.connect('dbname=deb2pg') as conn, \
            conn.cursor() as cur:

        cur.execute('insert into packages(name, version, arch, size_limit) values (%s, %s, %s, %s) returning id',
                (source_package, source_version, 'amd64', SIZE_LIMIT))
        pkg_id = cur.fetchone()

        fetch(source_package, source_version, destdir)
        pkgfolder = os.path.join(destdir, 'pkg')
        for dirpath, _, filelist in os.walk(pkgfolder):
            for f in filelist:
                full_name = os.path.join(dirpath, f)
                with open(full_name, 'rb') as fh:
                    rel_path = os.path.join(os.path.relpath(dirpath, pkgfolder), f)
                    if rel_path[0:2] == './':
                        rel_path = rel_path[2:]
                    ingest(pkg_id, fh, cur,
                            os.path.getsize(full_name),
                            rel_path)

def main(specs):
    for spec in specs:
        eat(*spec.split('=', 1))

if __name__ == '__main__':
    import sys
    main(sys.argv[1:])


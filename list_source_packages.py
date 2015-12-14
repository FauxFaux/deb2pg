#!/usr/bin/env python3
import os

import apt
from apt import apt_pkg


def list():
    src = apt_pkg.SourceRecords()
    acq = apt_pkg.Acquire(apt.progress.text.AcquireProgress())

    source_lookup = src.restart()

    while src.step():
        yield src.package + '=' + src.version


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


def main():
    for pkg in list():
        print(pkg)


if __name__ == '__main__':
    main()

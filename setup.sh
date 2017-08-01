#!/bin/bash
set -eux

# scp ../contentin/target/release/ci-gen target/release/deb2pg-ingest ingest.py setup.sh dxr1:
# ssh dxr1
# bash setup.sh

sudo mount -t tmpfs -o nodev,nosuid,size=10G tmpfs /tmp

sudo adduser --disabled-password faux
sudo mkdir ~faux/bin && sudo mv *-* ingest.py ~faux/bin && sudo chmod a+rx -R ~faux/bin

sudo apt update
sudo apt dist-upgrade -y
sudo apt install -y mdadm e2fsprogs xfsprogs postgresql python3

sudo umount /mnt || true
sudo mkdir /mnt/data

if [ -e /dev/nvme1n1 ]; then
    sudo mdadm --create --verbose /dev/md0 --level=stripe --raid-devices=2 /dev/nvme?n1
    sudo mkfs.xfs -K /dev/md0
    sudo mount -o nobarrier /dev/md0 /mnt/data
else
    sudo mkfs.xfs -K /dev/nvme0n1
    sudo mount -o nobarrier /dev/nvme0n1 /mnt/data
fi

for f in /sys/block/nvme*; do
    echo noop | sudo tee $f/queue/scheduler
done

echo 128000000 | sudo tee /proc/sys/kernel/sched_latency_ns

sudo chown faux:faux /mnt/data

#sudo apt install -y apt-mirror
# ... not doing apt-mirror, instead, attach volume and:
# sudo mkdir /mnt/mirror && sudo mount -o ro /dev/xvdf /mnt/mirror

#sudo apt install -y build-essential capnproto git curl
# ... not doing build. Tools already copied.

sudo systemctl stop postgresql
sleep 1 # sigh, racing shutdown
sudo mv /var/lib/postgresql/9.6/main /mnt/data/main
sudo ln -s /mnt/data/main /var/lib/postgresql/9.6/

echo '
shared_buffers = 1GB
max_wal_size = 128GB
work_mem = 50MB
maintenance_work_mem = 1GB
synchronous_commit = off

fsync = off
full_page_writes = off
' | sudo tee -a /etc/postgresql/9.?/main/postgresql.conf

sudo systemctl restart postgresql

# Monitoring tools:
sudo apt install -y aptitude awscli pigz ncdu iotop iftop linux-tools-$(uname -r)

sudo -u postgres createuser faux
sudo -u postgres createdb faux -O faux

# sudo su - faux
# rm ingest.log; rm ~/failure.log; time find /mnt/mirror -name \*.dsc -print0 | nice ionice xargs -0P16 -n20 python3 ~/bin/ingest.py 2>&1 | tee -a ingest.log

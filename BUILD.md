```
sudo adduser --disabled-password faux

sudo apt install -y aptitude apt-mirror build-essential git curl e2fsprogs python3 awscli postgresql pigz capnproto ncdu iotop linux-tools-$(uname -r) linux-tools-aws mdadm

sudo -u postgres createuser faux
sudo -u postgres createdb faux -O faux
```

```
# drive setup...
# small

sudo mkfs.ext4 /dev/nvme0n1
sudo mount -o nobarrier /dev/nvme0n1 /mnt/data

# big
mdadm --create --verbose /dev/md0 --level=stripe --raid-devices=2 /dev/nvme?n1
sudo mkfs.ext4 /dev/md0
sudo mount -o nobarrier /dev/md0 /mnt/data


sudo mkdir /mnt/data
sudo chown faux:faux /mnt/data

# ?
sudo mount /dev/xvdf /mnt/data/apt-mirror
```

/etc/apt/mirror.conf
```
set base_path    /mnt/data/apt-mirror
deb-src http://cloudfront.debian.net/debian jessie main contrib non-free
deb-src http://cloudfront.debian.net/debian stretch main contrib non-free

clean http://cloudfront.debian.net/debian
```

```
sudo systemctl stop postgresql
sudo mv /var/lib/postgresql/9.5/main /mnt/data/main
sudo ln -s /mnt/data/main /var/lib/postgresql/9.5/ 

shared_buffers = 1GB
max_wal_size = 128GB
work_mem = 50MB
maintenance_work_mem = 1GB
synchronous_commit = off


sudo su - faux
psql
... schema
```


```
# rust, as Faux
curl https://sh.rustup.rs -sSf | sh
mkdir /mnt/data/t
mkdir code
cd code
git clone https://github.com/FauxFaux/deb2pg
git clone https://github.com/FauxFaux/contentin
(cd deb2pg; cargo build --all --release)
(cd contentin; cargo build --all --release)
cd deb2pg
rm ingest.log; rm ~/failure.log; time find /mnt/data/apt-mirror -name \*.dsc -print0 | nice ionice xargs -0P8 -n20 python3 ingest.py 2>&1 | tee -a ingest.log
```


```
pg_dump | pigz > /mnt/data/t/all.sql.gz
aws s3 sync /mnt/data/t s3://dxr-1/t-2017-0723/
```

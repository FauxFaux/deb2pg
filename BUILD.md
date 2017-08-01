```
sudo adduser --disabled-password faux

sudo apt update
sudo apt dist-upgrade -y
sudo apt install -y aptitude apt-mirror build-essential git curl e2fsprogs xfsprogs python3 awscli postgresql pigz capnproto ncdu iotop linux-tools-$(uname -r) mdadm iftop

sudo -u postgres createuser faux
sudo -u postgres createdb faux -O faux
```

```
for f in /sys/block/*; do echo noop | sudo tee $f/queue/scheduler; done
echo 128000000 | sudo tee /proc/sys/kernel/sched_latency_ns
sudo mount -t tmpfs -o nodev,nosuid,size=10G tmpfs /tmp
```

```
# drive setup...
sudo mkdir /mnt/data


# small

# sudo mkfs.ext4 -E nodiscard /dev/nvme0n1
sudo mkfs.xfs -K /dev/nvme0n1
sudo mount -o nobarrier /dev/nvme0n1 /mnt/data

# big
sudo umount /mnt
sudo mdadm --create --verbose /dev/md0 --level=stripe --raid-devices=2 /dev/xvdb /dev/xvdc
sudo mdadm --create --verbose /dev/md0 --level=stripe --raid-devices=2 /dev/nvme?n1
# sudo mkfs.ext4 -E nodiscard /dev/md0
sudo mkfs.xfs -K /dev/md0
sudo mount -o nobarrier /dev/md0 /mnt/data

sudo chown faux:faux /mnt/data

# ?
sudo mkdir /mnt/mirror && sudo mount -o ro /dev/xvdf /mnt/mirror
```

/etc/apt/mirror.list
```
set base_path    /mnt/data/apt-mirror
deb-src http://cloudfront.debian.net/debian jessie main contrib non-free
deb-src http://cloudfront.debian.net/debian stretch main contrib non-free

clean http://cloudfront.debian.net/debian
```

```
sudo systemctl stop postgresql
sudo mv /var/lib/postgresql/9.6/main /mnt/data/main
sudo ln -s /mnt/data/main /var/lib/postgresql/9.6/ 

sudo vim /etc/postgresql/9.?/main/postgresql.conf
shared_buffers = 1GB
max_wal_size = 128GB
work_mem = 50MB
maintenance_work_mem = 1GB
synchronous_commit = off

# later considered:
fsync = off
full_page_writes = off


sudo systemctl restart postgresql

sudo su - faux
psql
... schema
```


```
# rust, as Faux
curl https://sh.rustup.rs -sSf | sh
.. customise
.. nightly
.. yes yes yes

.. exit and su again

mkdir /mnt/data/t ~/bin
# scp ~/code/contentin/target/release/ci-gen ~/code/deb2pg/target/release/deb2pg-ingest ~/code/deb2pg/ingest.py dxr1:
# sudo mv *-* ingest.py ~faux/bin && sudo chmod a+rx -R ~faux/bin


mkdir code
cd code
git clone https://github.com/FauxFaux/deb2pg
git clone https://github.com/FauxFaux/contentin
(cd deb2pg; cargo build --all --release)
(cd contentin; cargo build --all --release)
cd deb2pg
rm ingest.log; rm ~/failure.log; time find /mnt/mirror -name \*.dsc -print0 | nice ionice xargs -0P16 -n20 python3 ingest.py 2>&1 | tee -a ingest.log
```


```
pg_dump | pigz > /mnt/data/t/all.sql.gz
aws s3 sync /mnt/data/t s3://dxr-1/t-2017-0723/
```

### astoria (4, 24gb, raid0 ssd)

52gb in ~2h30m. Probably CPU limited, haven't run it recently.

### i3.xlarge (4v, 30gb, single 1t nvme)

Lots of system time, but generally cpu or parallelism limited.

~5gb in 15 minutes -> ~ same speed as `astoria`.

### c3.8xlarge (32v, 60gb, raid0 ssd):

Claims lots of idle cpu, even at -P24, don't really understand.
Maybe `find` is failing to scan the mirror? Or it's getting destroyed by all the context switching.
iotop thinks there's pretty consistent 100-200MB/s write, from postgres and ingesters, so we could be limited
by the raid0 write performance.

Load-average is 20-40, for 32vCPU.
```
20  1            0     12018380       316136     47256032    0    0  3152 143780 236137 215621  15  21  62   2   1
19  1            0     11845284       317776     47358560    0    0  4916 91144 266727 253287   19  25  54   1   1
39  0            0     11843756       318992     47388992    0    0  8392 114000 215370 200191  18  25  55   1   1
11  2            0     11924312       319484     47281664    0    0 24716 108352 287441 282828  20  18  57   5   1
12  2            0     11746912       319616     47473792    0    0  9008 146168 154100 143671  10   7  77   6   0
17  1            0     11669740       319748     47538852    0    0 23456 126520 186576 168356  15  17  63   4   0
 1  1            0     11828704       319892     47328888    0    0 23436 102808 224288 197295  21  27  51   1   0
24  1            0     11767000       320892     47378320    0    0 13988 103428 180149 165000  13  23  61   2   0
```

Seems to have done ~11gb in 15 minutes, -> twice as fast.

Turning off fsync in postgres gets us closer to 6gb in 6 minutes.

ps aux | fgrep -- -ingest
sudo strace -T -yy -p 62996 2>&1 | fgrep -v '<0.00'

... this makes it look like `open()` is slow, ew. Doesn't even mention the flocking,
which is also probably slow.

```
strace: Process 62996 attached
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.016896>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.161907>
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.010586>
open("/mnt/data/t//text-3.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000000> <0.011101>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.194234>
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.011327>
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.011499>
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.163705>
open("/mnt/data/t//text-3.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000000> <0.029028>
recvfrom(3<UNIX:[904676->905775]>, "2\0\0\0\4D\0\0\0\n\0\1\0\0\0\0C\0\0\0\rSELECT 1\0Z\0"..., 8192, 0, NULL, NULL) = 36 <0.012198>
open("/mnt/data/t//text-3.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000000> <0.011672>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.023650>
open("/mnt/data/t//text-3.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000000> <0.020053>
open("/mnt/data/t//text-3.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000000> <0.027389>
open("/mnt/data/t//text-3.0000000000000000000001", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-3.0000000000000000000001> <0.019085>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.030680>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.039643>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.033958>
open("/mnt/data/t//text-2.0000000000000000000000", O_WRONLY|O_CREAT|O_CLOEXEC, 0666) = 6</mnt/data/t/text-2.0000000000000000000000> <0.038228>

```

### i3.4xlarge (16v, 120gb, dual 2t nvme)

Race in stop postgres && mv. Rage. Had to re-cluser.

mkfs.ext4 took forever (1% done after a minute?) without `-E nodiscard`.
With that, `iotop` reports it lazy initing, so it's probably sucking IO.

At `-P16`, we're getting very minimal IO wait, and some claimed idle CPU; perhaps being lost to `sys`,
I suspect again that `sys` is the above `open()` failure, which might not be anything to do with
poor IO (it would be cached anyway, right?).

Disadvantages here: Actually using the mirror off the mount, and /mnt/data isn't mounted nobarrier.
Totally awful test.

```
13  0            0     90590272       348888     33100524    0    0  8064 175404 249677 275997  30  23  45   0   2
 8  0            0     90565944       349244     33140572    0    0 20080 180792 250437 280279  30  23  44   0   2
16  0            0     90283432       349636     33189112    0    0 21032 419488 213348 259659  31  24  43   0   2
 2  1            0     90319168       349996     33260248    0    0 20632 171836 200920 247008  43  22  33   0   2
```

Also doing about 1gb/minute. Definitely need to fix that open()/flock() thing. Maybe it's also causing the idle.

### i3.4xlarge (16v, 120gb, dual 2t nvme; tweaking)

Took ~30s for the spot request to be fulfilled, not unexpected given the bid price
defaulting to the on-demand price.

Tweaking:
 * zesty (ami-254ba35c), so new kernel with nvme fixes, and postgres 9.6.
   * https://cloud-images.ubuntu.com/locator/ec2/ "hvm:ebs-ssd zesty eu-west-1"
 * xfs default settings
 * noop scheduler (as recommended by xfs)
 * increased scheduler max latency to 128ms.
 * new code with reduced re-opening of files and locking
 * temp dir moved to subdirectory, hoping to reduce inode contention

But, also, no idea what I was measuring before. Total packs, or just text?
Total seems more reasonable. It's also what the 106gb total is measured in.

Bugs:
 * Postgres reporting deadlocks in path_component
 * Very, very occasional blob_pos collisions; blah. Kinda seeing these locally, too.

Maybe /tmp is slow, slowing down ci-gen?
I hope nothing ever gets flushed before getting deleted.
Just in case:

```
sudo mount -t tmpfs -o nodev,nosuid,size=10G tmpfs /tmp
```

-P16:
Still managed to generate ~30GB total of packs in 15 minutes. -> 53 minutes. \o/
Sub hour, oh yeah, oh yeah.

~48GB in 30 minutes -> 66 minutes. Blah.

```
ubuntu@ip-172-31-37-162:~$ sudo iostat -m
Linux 4.10.0-28-generic (ip-172-31-37-162) 	07/30/2017 	_x86_64_	(16 CPU)

avg-cpu:  %user   %nice %system %iowait  %steal   %idle
           9.44   21.99   19.13    1.22    2.40   45.82

Device:            tps    MB_read/s    MB_wrtn/s    MB_read    MB_wrtn
xvda             84.20         0.12         8.03        258      17858
nvme0n1        2316.89         0.01        44.55         32      99105
nvme1n1        2313.36         0.01        44.54         32      99081
md0            6640.48         0.03        89.08         56     198187
xvdf            154.61        17.28         0.09      38448        209
```



-P8 shows a lot more idle CPU in `vmstat -w 10`:
```
12  0            0     78835288        42912     45199284    0    0  6717 67064 261781 276595  31  23  43   1   2
 9  0            0     78290192        42916     45635940    0    0  8741 82268 290660 299943  29  20  47   1   2
``` 

-P24 still has lots of idle; must be concurrency fail, not bad utilisation:
```
24  0            0     75418448        43224     46668904    0    0  4034 77007 241940 264895  36  20  40   1   2
18  0            0     75130008        43228     47006564    0    0  9236 71527 199460 226152  37  23  37   1   2
```


When are we leaving temp files behind? ?? directories have thousands of files in
*each* and take ages to cleanup between test runs.

Instance doesn't automatically stop at the end of a defined-duration lease, oops.

### i3.4xlarge (16v, 120gb, dual 2t nvme; tweaking)

And again, this time without deadlocky `path_component` code,
 and with a better attempt at unlinking hash files.. seems.. not faster.
 
Might be time to dump this mutli-phase process, it's essentially pointless, right? 
 
unlinking is actually pretty slow, 0.5-14ms typically. In a hot loop. Bastards.
1ms * 1_000 files is a whole second wasted.

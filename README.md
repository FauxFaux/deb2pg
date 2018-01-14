Corpus
------

 * deb-src jessie main contrib non-free
 * deb-src stretch main contrib non-free


 * 40,856 containers.

 
 * 10,125,657 path components. 2GB including an index per column.
 * 11,134,515 blobs. 1.7GB including h0 and pos indexes.
 * 20,138,708 files. 2.7GB with only PKEY.


Ingest 2
--------
```
T=/mnt/data/t; rm -rf $T/packs $T/loose; mkdir -p $T/packs $T/loose $T/packs/{0..9} $T/packs/{a..f}; (cd $T/loose && mkdir -p $(printf "%03x " {0..4095}))

```

Zstd / sizes
------------

50 random source packages, happened to include `llvm-toolchain`,
`thunderbird` and `php7`. https://b.goeswhere.com/source-text.tar.lzma

Took all text files from the `apt-get source` result (i.e. not fully unpacked,
patched, debian/ folder present), and worked out some statistics on them.
Given around 30,000 packages, 50 is not a great sample, so multiplying these
numbers by 600 is not ideal. Relative sizes of source files should be representative,
though.

 * zstd dictionaries, trained on files 1-99, 100-999, and 1,000-9,999 bytes.
 * Sizes after default zstd compression. Of 256507 files:
   * max: 2,000kb
   * 0.01%  (25) are >320kb
   * 0.02%  (51) are >250kb
   * 0.05% (128) are > 99kb
   * 0.11% (282) are > 64kb.
   * 0.33% (864) are > 32kb.
   * min: 13 bytes.

1/0.33% = 300.
1/0.11% = 900.

13-byte minimum comes from checksum (4 bytes) (`--no-check`), and a 4-byte magic number,
so ~6 bytes for a single-byte input. Not a super interesting case.

Ingest
------

Ingest on `astoria` takes about 2h30m on a 90GB mirror, giving 106GB of packs:

```
find /mnt/data/sources -name \*.dsc -print0  0.26s user 0.45s system 0% cpu 2:25:46.29 total
nice ionice xargs -0P4 -n20 python3 ingest.py 2>&1  10840.38s user 4274.89s system 167% cpu 2:30:14.45 total
tee -a ingest.log  0.02s user 0.15s system 0% cpu 2:30:14.45 total
```

Machine is generally responsive while running this, probably due
  to very limited use of IO on `/`.

Lots of idle CPU, but running at a higher `-P` doesn't feel productive.


Indexing
--------

Indexing makes the machine jerk and jump, possibly due to the heavy `/tmp` (real fs)
  IO, mostly via. mmap.

```
nice ionice make -j 4 -f ~/code/deb2pg/reindex/Makefile.index ${a[@]/%/.idx}  6321.71s user 138.21s system 333% cpu 32:18.24 total
```


Failures
--------

Hopefully some of these are timeouts. zip might be zip64, which we don't seem
  to do a good job on still.

 * android-platform-libcore/android-platform-libcore_7.0.0+r33-1.dsc
 * apbs/apbs_1.4-1.dsc
 * chromium-browser/chromium-browser_57.0.2987.98-1~deb8u1.dsc
 * chromium-browser/chromium-browser_59.0.3071.86-1.dsc
 * dmaths/dmaths_3.4+dfsg1-1.dsc
 * eclipse-rse/eclipse-rse_3.4.2-1.dsc
 * erlang/erlang_17.3-dfsg-4+deb8u1.dsc
 * erlang/erlang_19.2.1+dfsg-2.dsc
 * espresso/espresso_6.0-3.dsc
 * flightgear-data/flightgear-data_2016.4.2+dfsg-1.dsc
 * flightgear-data/flightgear-data_3.0.0-3.dsc
 * fuse-zip/fuse-zip_0.4.0-1.dsc
 * fuse-zip/fuse-zip_0.4.0-2.dsc
 * gamera/gamera_3.4.1+svn1423-4.dsc
 * gamera/gamera_3.4.2+git20160808.1725654-1.dsc
 * gcc-4.9/gcc-4.9_4.9.1-3.dsc
 * gts/gts_0.7.6+darcs121130-4.dsc
 * ktorrent/ktorrent_4.3.1-2.dsc
 * libacpi/libacpi_0.2-4.dsc
 * libcommons-compress-java/libcommons-compress-java_1.13-1.dsc
 * libimage-exiftool-perl/libimage-exiftool-perl_10.40-1.dsc
 * libkml/libkml_1.3.0-3.dsc
 * libkml/libkml_1.3.0~r864+dfsg-1.dsc
 * libreoffice/libreoffice_4.3.3-2+deb8u7.dsc
 * libtritonus-java/libtritonus-java_20070428-10.dsc
 * libzip/libzip_0.11.2-1.2.dsc
 * libzip/libzip_1.1.2-1.1.dsc
 * linux/linux_3.16.43-1.dsc
 * mauve/mauve_20140821-1.dsc
 * mediawiki/mediawiki_1.27.3-1.dsc
 * node-yauzl/node-yauzl_2.1.0-1.dsc
 * nvram-wakeup/nvram-wakeup_1.1-1.dsc
 * nvram-wakeup/nvram-wakeup_1.1-4.dsc
 * openarena-088-data/openarena-088-data_0.8.8-2.dsc
 * php5/php5_5.6.30+dfsg-0+deb8u1.dsc
 * php7.0/php7.0_7.0.19-1.dsc
 * qtxmlpatterns-opensource-src/qtxmlpatterns-opensource-src_5.3.2-2.dsc
 * rekall/rekall_1.6.0+dfsg-2.dsc
 * scotch/scotch_5.1.12b.dfsg-2.dsc
 * texlive-extra/texlive-extra_2014.20141024-1.dsc
 * texlive-extra/texlive-extra_2016.20170123-5.dsc
 * util-linux/util-linux_2.25.2-6.dsc
 * util-linux/util-linux_2.29.2-1.dsc
 * vboot-utils/vboot-utils_0~R52-8350.B-2.dsc

... + [e2fsprogs](https://github.com/FauxFaux/ext4-rs/issues/1).

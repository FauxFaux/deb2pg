create type git_mode as enum ('100644', '100755', '120000');

create table blobs (
    hash varchar primary key,
    content varchar not null
);

create table files (
    package bigserial not null,
    mode git_mode not null,
    path varchar not null,
    hash varchar
);

create unique index package_paths on files (package, path);

create table packages (
    id bigserial primary key,
    name varchar not null,
    version varchar not null,
    arch varchar not null,
    size_limit bigint not null
);

create unique index package_name_version_arch on packages(name, version, arch);

create view package_file_contents as select packages.name,packages.version,files.path,files.hash,blobs.content from packages inner join files on (packages.id=files.package) inner join blobs on (blobs.hash = files.hash);

create index content_contains_date on blobs ((content like '%date %'));
create index file_hash on files using hash (hash);


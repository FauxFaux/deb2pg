create table blobs (
    hash varchar primary key,
    content varchar not null
);

create table files (
    package bigserial not null,
    path varchar not null,
    hash varchar
);

create unique index package_paths on files (package, path);

create table packages (
    id bigserial primary key,
    name varchar not null,
    version varchar not null,
    arch varchar not null
);

create unique index package_name_version_arch on packages(name, version, arch);


CREATE TYPE git_mode AS ENUM ('100644', '100755', '120000');

CREATE TABLE blobs (
  hash_prefix UUID PRIMARY KEY,
  hash_suffix INT     NOT NULL,
  content     VARCHAR NOT NULL
);

CREATE TABLE files (
  package     BIGINT   NOT NULL,
  mode        git_mode NOT NULL,
  hash_prefix UUID,
  path        VARCHAR  NOT NULL
);

CREATE UNIQUE INDEX package_paths ON files (package, path);

CREATE TABLE packages (
  id         BIGSERIAL PRIMARY KEY,
  size_limit BIGINT  NOT NULL,
  name       VARCHAR NOT NULL,
  version    VARCHAR NOT NULL
);

CREATE UNIQUE INDEX package_name_version_arch ON packages (name, version);

CREATE VIEW package_file_contents AS
  SELECT
    packages.name,
    packages.version,
    files.path,
    files.hash,
    blobs.content
  FROM packages
    INNER JOIN files ON (packages.id = files.package)
    INNER JOIN blobs ON (blobs.hash = files.hash);

SELECT
  packages.name,
  packages.version,
  files.path,
  files.hash,
  blobs.content
FROM packages
  JOIN files ON packages.id = files.package
  JOIN blobs ON blobs.hash :: TEXT = files.hash :: TEXT;


CREATE INDEX content_contains_date ON blobs ((content LIKE '%date %'));
CREATE INDEX file_hash ON files USING HASH (hash);

-- https://stackoverflow.com/questions/8316164/convert-hex-in-text-representation-to-decimal-number
EXPLAIN ANALYZE UPDATE blobs
SET hash_prefix = lpad(hash, 32, '0') :: UUID,
  hash_suffix   = (('x' || lpad(substring(hash FROM 33), 8, '0')) :: BIT(32) :: INT);


TRUNCATE TABLE packages;
TRUNCATE TABLE files;
TRUNCATE TABLE blobs;
--migration first pass at tables

CREATE TABLE blob (
  len BIGINT NOT NULL,
  h0  BIGINT NOT NULL,
  h1  BIGINT NOT NULL,
  h2  BIGINT NOT NULL,
  h3  BIGINT NOT NULL,
  pos BIGINT
);

CREATE INDEX blob_h0_key ON blob (h0);
CREATE UNIQUE INDEX blob_pos ON blob(pos) WHERE pos IS NOT NULL;

CREATE TABLE path_component (
  id   BIGSERIAL PRIMARY KEY,
  path VARCHAR NOT NULL,
  UNIQUE (path)
);

CREATE TABLE file (
  id        BIGSERIAL PRIMARY KEY,
  container BIGINT    NOT NULL,
  pos       BIGINT    NOT NULL,
  paths     BIGINT [] NOT NULL
);

CREATE INDEX file_pos ON file (pos);

CREATE TABLE container (
  id       BIGSERIAL PRIMARY KEY,
  ingested TIMESTAMPTZ NOT NULL DEFAULT now(),
  info     JSONB       NOT NULL
);

CREATE SEQUENCE loose_blob_seq;

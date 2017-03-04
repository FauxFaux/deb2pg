--migration first pass at tables

CREATE TABLE blob (
  len BIGINT NOT NULL,
  h0  BIGINT NOT NULL,
  h1  BIGINT NOT NULL,
  h2  BIGINT NOT NULL,
  h3  BIGINT NOT NULL,
  pos BIGINT,
  UNIQUE (h0, h1, h2, h3)
);

CREATE TABLE path_component (
  id   BIGSERIAL PRIMARY KEY,
  path VARCHAR NOT NULL
);

CREATE TABLE file (
  id        BIGSERIAL PRIMARY KEY,
  container BIGINT    NOT NULL,
  pos       BIGINT    NOT NULL,
  paths     BIGINT [] NOT NULL
);

CREATE TABLE container (
  id   BIGSERIAL PRIMARY KEY,
  name VARCHAR NOT NULL
);

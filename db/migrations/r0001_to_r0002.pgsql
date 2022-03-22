CREATE TABLE wordle_archive.schema_version
( schema_version bigint NOT NULL
, CONSTRAINT pkey__schema_version PRIMARY KEY (schema_version)
);

INSERT INTO wordle_archive.schema_version (schema_version) VALUES (2);

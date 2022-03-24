CREATE SCHEMA wordle_archive;

CREATE SEQUENCE wordle_archive.seq__sites__id AS bigint;

CREATE TABLE wordle_archive.sites
( id bigint NOT NULL DEFAULT nextval('wordle_archive.seq__sites__id')
, name character varying(128) NOT NULL
, url character varying(128) NOT NULL
, css_class character varying(128) NOT NULL
, variant character varying(32) NOT NULL
, CONSTRAINT pkey__sites PRIMARY KEY (id)
, CONSTRAINT uq__sites__name UNIQUE (name)
, CONSTRAINT uq__sites__url UNIQUE (url)
);

CREATE SEQUENCE wordle_archive.seq__puzzles__id AS bigint;

CREATE TABLE wordle_archive.puzzles
( id bigint NOT NULL DEFAULT nextval('wordle_archive.seq__puzzles__id')
, site_id bigint NOT NULL
, puzzle_date date NOT NULL
, day_ordinal bigint NOT NULL DEFAULT 0
, head text NOT NULL
, tail text NOT NULL
, pattern text NOT NULL
, solution text NOT NULL
, attempts bigint NULL
, raw_pattern text NULL DEFAULT NULL
, CONSTRAINT pkey__puzzles PRIMARY KEY (id)
, CONSTRAINT fkey__puzzles__site_id FOREIGN KEY (site_id) REFERENCES wordle_archive.sites(id)
, CONSTRAINT uq__puzzles__site_puzzle_day UNIQUE (site_id, puzzle_date, day_ordinal)
);

CREATE VIEW wordle_archive.sites_and_puzzles AS
    SELECT
        s.id site_id,
        s.name site_name,
        s.url site_url,
        s.css_class site_css_class,
        s.variant,
        p.id puzzle_id,
        p.puzzle_date,
        p.day_ordinal,
        p.head,
        p.tail,
        p.pattern,
        p.solution,
        p.attempts,
        p.raw_pattern
    FROM
        wordle_archive.sites s
        INNER JOIN wordle_archive.puzzles p
            ON p.site_id = s.id
;

CREATE TABLE wordle_archive.schema_version
( schema_version bigint NOT NULL
, CONSTRAINT pkey__schema_version PRIMARY KEY (schema_version)
);

INSERT INTO wordle_archive.schema_version (schema_version) VALUES (7);

CREATE VIEW wordle_archive.site_stats AS
    SELECT
        s.id site_id,
        s.name site_name,
        s.url site_url,
        s.css_class site_css_class,
        s.variant,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles vic
            WHERE vic.site_id = s.id AND vic.attempts IS NOT NULL
        ) puzzles_won,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles los
            WHERE los.site_id = s.id AND los.attempts IS NULL
        ) puzzles_lost,
        (
            SELECT CAST(AVG(avr.attempts) AS double precision)
            FROM wordle_archive.puzzles avr
            WHERE avr.site_id = s.id
        ) average_attempts
    FROM wordle_archive.sites s
;

CREATE VIEW wordle_archive.variant_stats AS
    SELECT DISTINCT
        s.variant,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles vic
            INNER JOIN wordle_archive.sites vics ON vics.id = vic.site_id
            WHERE vics.variant = s.variant AND vic.attempts IS NOT NULL
        ) puzzles_won,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles los
            INNER JOIN wordle_archive.sites loss ON loss.id = los.site_id
            WHERE loss.variant = s.variant AND los.attempts IS NULL
        ) puzzles_lost,
        (
            SELECT CAST(AVG(avr.attempts) AS double precision)
            FROM wordle_archive.puzzles avr
            INNER JOIN wordle_archive.sites avrs ON avrs.id = avr.site_id
            WHERE avrs.variant = s.variant
        ) average_attempts
    FROM wordle_archive.sites s
;

CREATE VIEW wordle_archive.global_stats AS
    SELECT
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles vic
            WHERE vic.attempts IS NOT NULL
        ) puzzles_won,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles los
            WHERE los.attempts IS NULL
        ) puzzles_lost,
        (
            SELECT CAST(AVG(avr.attempts) AS double precision)
            FROM wordle_archive.puzzles avr
        ) average_attempts
;

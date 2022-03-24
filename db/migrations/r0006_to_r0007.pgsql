ALTER TABLE wordle_archive.puzzles ADD COLUMN raw_pattern text NULL DEFAULT NULL;

CREATE OR REPLACE VIEW wordle_archive.sites_and_puzzles AS
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

UPDATE wordle_archive.schema_version SET schema_version=7;

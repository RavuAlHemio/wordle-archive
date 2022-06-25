ALTER TABLE wordle_archive.sites ADD COLUMN ordering bigint NOT NULL DEFAULT 0;

DROP VIEW wordle_archive.sites_and_puzzles;
CREATE VIEW wordle_archive.sites_and_puzzles AS
    SELECT
        s.id site_id,
        s.name site_name,
        s.url site_url,
        s.css_class site_css_class,
        s.variant,
        s.notes,
        s.available,
        s.ordering,
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

DROP VIEW wordle_archive.site_stats;
CREATE VIEW wordle_archive.site_stats AS
    SELECT
        s.id site_id,
        s.name site_name,
        s.url site_url,
        s.css_class site_css_class,
        s.variant,
        s.ordering,
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
        ) average_attempts,
        (
            SELECT wordle_archive.site_longest_streak(s.id)
        ) longest_streak,
        (
            SELECT wordle_archive.site_current_streak(s.id)
        ) current_streak
    FROM wordle_archive.sites s
;

UPDATE wordle_archive.schema_version SET schema_version=12;

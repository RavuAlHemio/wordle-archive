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

UPDATE wordle_archive.schema_version SET schema_version=6;

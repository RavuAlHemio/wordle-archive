-- executed by migration if the column victory in wordle_archive.puzzles does not yet exist:
-- ALTER TABLE wordle_archive.puzzles ADD COLUMN victory boolean NOT NULL DEFAULT FALSE;
-- ALTER TABLE wordle_archive.puzzles ALTER COLUMN victory DROP DEFAULT;

-- executed by migration if the column victory in wordle_archive.sites_and_puzzles does not yet exist:
-- CREATE OR REPLACE VIEW wordle_archive.sites_and_puzzles AS
--     SELECT
--         s.id site_id,
--         s.name site_name,
--         s.url site_url,
--         s.css_class site_css_class,
--         s.variant,
--         p.id puzzle_id,
--         p.puzzle_date,
--         p.day_ordinal,
--         p.head,
--         p.tail,
--         p.pattern,
--         p.solution,
--         p.victory
--     FROM
--         wordle_archive.sites s
--         INNER JOIN wordle_archive.puzzles p
--             ON p.site_id = s.id
-- ;

-- existing puzzles' victory state is then updated

-- executed by migration once finished:
-- UPDATE wordle_archive.schema_version SET schema_version=3;

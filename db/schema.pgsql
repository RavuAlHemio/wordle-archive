CREATE SCHEMA wordle_archive;

CREATE SEQUENCE wordle_archive.seq__sites__id AS bigint;

CREATE TABLE wordle_archive.sites
( id bigint NOT NULL DEFAULT nextval('wordle_archive.seq__sites__id')
, name character varying(128) NOT NULL
, url character varying(128) NOT NULL
, css_class character varying(128) NOT NULL
, variant character varying(32) NOT NULL
, notes text NOT NULL DEFAULT ''
, available boolean NOT NULL DEFAULT TRUE
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
        s.notes,
        s.available,
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

INSERT INTO wordle_archive.schema_version (schema_version) VALUES (10);

CREATE FUNCTION wordle_archive.site_streaks(streak_site_id bigint) RETURNS TABLE(streak bigint, victory boolean) AS $$
DECLARE
    puzzle_victory boolean;
    streak_row record;
BEGIN
    streak := 0;
    victory := FALSE;
    FOR streak_row IN
        SELECT attempts
        FROM wordle_archive.puzzles
        WHERE site_id = streak_site_id
        ORDER BY puzzle_date, day_ordinal
    LOOP
        puzzle_victory := CASE WHEN streak_row.attempts IS NULL THEN FALSE ELSE TRUE END;
        IF victory <> puzzle_victory
        THEN
            -- streak ended; output it
            IF streak > 0
            THEN
                RETURN NEXT;
                streak := 0;
            END IF;
            victory := puzzle_victory;
        END IF;
        streak := streak + 1;
    END LOOP;

    -- output final streak
    IF streak > 0
    THEN
        RETURN NEXT;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.site_current_streak(streak_site_id bigint) RETURNS bigint AS $$
DECLARE
    current_streak bigint := 0;
    streak_row record;
BEGIN
    FOR streak_row IN SELECT streak, victory FROM wordle_archive.site_streaks(streak_site_id)
    LOOP
        IF streak_row.victory
        THEN
            current_streak := streak_row.streak;
        ELSE
            current_streak := 0;
        END IF;
    END LOOP;
    RETURN current_streak;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.site_longest_streak(streak_site_id bigint) RETURNS bigint AS $$
DECLARE
    max_streak bigint := 0;
    streak_row record;
BEGIN
    FOR streak_row IN SELECT streak, victory FROM wordle_archive.site_streaks(streak_site_id)
    LOOP
        IF streak_row.victory AND max_streak < streak_row.streak
        THEN
            max_streak := streak_row.streak;
        END IF;
    END LOOP;
    RETURN max_streak;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.variant_longest_streak(streak_variant character varying(32)) RETURNS bigint AS $$
DECLARE
    max_streak bigint := 0;
    streak_site_id bigint;
    streak_value bigint;
BEGIN
    FOR streak_site_id IN SELECT id FROM wordle_archive.sites WHERE variant = streak_variant
    LOOP
        SELECT wordle_archive.site_longest_streak(streak_site_id) INTO streak_value;
        IF max_streak < streak_value
        THEN
            max_streak := streak_value;
        END IF;
    END LOOP;
    RETURN max_streak;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.global_longest_streak() RETURNS bigint AS $$
DECLARE
    max_streak bigint := 0;
    streak_site_id bigint;
    streak_value bigint;
BEGIN
    FOR streak_site_id IN SELECT id FROM wordle_archive.sites
    LOOP
        SELECT wordle_archive.site_longest_streak(streak_site_id) INTO streak_value;
        IF max_streak < streak_value
        THEN
            max_streak := streak_value;
        END IF;
    END LOOP;
    RETURN max_streak;
END;
$$ LANGUAGE plpgsql;

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
        ) average_attempts,
        (
            SELECT wordle_archive.site_longest_streak(s.id)
        ) longest_streak,
        (
            SELECT wordle_archive.site_current_streak(s.id)
        ) current_streak
    FROM wordle_archive.sites s
;

CREATE VIEW wordle_archive.variant_stats AS
    WITH variants(variant) AS (
        SELECT DISTINCT s.variant FROM wordle_archive.sites s
    )
    SELECT DISTINCT
        v.variant,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles vic
            INNER JOIN wordle_archive.sites vics ON vics.id = vic.site_id
            WHERE vics.variant = v.variant AND vic.attempts IS NOT NULL
        ) puzzles_won,
        (
            SELECT CAST(COUNT(*) AS bigint)
            FROM wordle_archive.puzzles los
            INNER JOIN wordle_archive.sites loss ON loss.id = los.site_id
            WHERE loss.variant = v.variant AND los.attempts IS NULL
        ) puzzles_lost,
        (
            SELECT CAST(AVG(avr.attempts) AS double precision)
            FROM wordle_archive.puzzles avr
            INNER JOIN wordle_archive.sites avrs ON avrs.id = avr.site_id
            WHERE avrs.variant = v.variant
        ) average_attempts,
        (
            SELECT wordle_archive.variant_longest_streak(v.variant)
        ) longest_streak
    FROM variants v
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
        ) average_attempts,
        (
            SELECT wordle_archive.global_longest_streak()
        ) longest_streak
;

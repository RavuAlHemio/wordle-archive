DROP VIEW wordle_archive.site_stats;
DROP VIEW wordle_archive.variant_stats;
DROP VIEW wordle_archive.global_stats;

CREATE FUNCTION wordle_archive.site_streaks(streak_site_id bigint) RETURNS SETOF bigint AS $$
DECLARE
    streak bigint := 0;
    streak_row record;
BEGIN
    FOR streak_row IN
        SELECT attempts
        FROM wordle_archive.puzzles
        WHERE site_id = streak_site_id
        ORDER BY puzzle_date, day_ordinal
    LOOP
        IF streak_row.attempts IS NULL THEN
            -- streak ended; output it
            IF streak > 0 THEN
                RETURN NEXT streak;
                streak := 0;
            END IF;
        ELSE
            streak := streak + 1;
        END IF;
    END LOOP;

    -- output final streak
    IF streak > 0 THEN
        RETURN NEXT streak;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.site_current_streak(streak_site_id bigint) RETURNS bigint AS $$
DECLARE
    current_streak bigint := 0;
    streak_value bigint;
BEGIN
    FOR streak_value IN SELECT wordle_archive.site_streaks(streak_site_id)
    LOOP
        current_streak := streak_value;
    END LOOP;
    RETURN current_streak;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION wordle_archive.site_longest_streak(streak_site_id bigint) RETURNS bigint AS $$
DECLARE
    max_streak bigint := 0;
    streak_value bigint;
BEGIN
    FOR streak_value IN SELECT wordle_archive.site_streaks(streak_site_id)
    LOOP
        IF max_streak < streak_value
        THEN
            max_streak := streak_value;
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

UPDATE wordle_archive.schema_version SET schema_version=8;

-- change of return type requires DROP and re-CREATE
DROP FUNCTION wordle_archive.site_streaks;
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

-- these can be replaced
CREATE OR REPLACE FUNCTION wordle_archive.site_current_streak(streak_site_id bigint) RETURNS bigint AS $$
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

CREATE OR REPLACE FUNCTION wordle_archive.site_longest_streak(streak_site_id bigint) RETURNS bigint AS $$
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

UPDATE wordle_archive.schema_version SET schema_version=9;

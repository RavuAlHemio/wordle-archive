GRANT USAGE ON SCHEMA wordle_archive TO {user};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA wordle_archive TO {user};
GRANT SELECT ON ALL VIEWS IN SCHEMA wordle_archive TO {user};
GRANT SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA wordle_archive TO {user};

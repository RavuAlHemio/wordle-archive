mod migrations_r0001;
mod migrations_r0006;
pub(crate) mod migration_utils;


use std::collections::HashSet;
use std::fmt::Debug;

use async_trait::async_trait;
use chrono::NaiveDate;
use log::error;
use tokio_postgres::{self, NoTls};

use crate::config::CONFIG;
use crate::model::{Puzzle, PuzzleSite, SiteAndPuzzle, Stats, StatsSubject};


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum OptionResult<T> {
    Present(T),
    Absent,
    Error,
}


pub(crate) struct DbConnection {
    client: tokio_postgres::Client,
}
impl DbConnection {
    pub async fn new() -> Option<Self> {
        let conn_string = {
            let config_guard = CONFIG
                .get().expect("no CONFIG set")
                .read().await;
            config_guard.db_conn_string.clone()
        };

        let (client, connection) = match tokio_postgres::connect(&conn_string, NoTls).await {
            Ok(cc) => cc,
            Err(e) => {
                error!("error connecting to database: {}", e);
                return None;
            },
        };

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("database connection error: {}", e);
            }
        });

        // pre-fetch current schema version
        let mut current_schema_version = None;
        {
            let row_res = client.query_one(
                "SELECT schema_version FROM wordle_archive.schema_version",
                &[],
            ).await;
            if let Ok(row) = row_res {
                let ver: i64 = row.get(0);
                current_schema_version = Some(ver);
            }
            // fail silently
        }

        // run migrations
        let current_migrations: [&(dyn DbMigration); 6] = [
            &migrations_r0001::MigrationR0001ToR0002,
            &migrations_r0001::MigrationR0002ToR0003,
            &migrations_r0001::MigrationR0003ToR0004,
            &migrations_r0001::MigrationR0004ToR0005,
            &migrations_r0001::MigrationR0005ToR0006,
            &migrations_r0006::MigrationR0006ToR0007,
        ];
        for migration in current_migrations {
            match migration.is_required(&client, current_schema_version).await {
                Ok(false) => continue,
                Ok(true) => {
                    if !migration.migrate(&client).await {
                        // failure information has already been logged
                        return None;
                    }
                },
                Err(e) => {
                    error!("failed to ascertain whether migration {:?} is necessary: {}", migration, e);
                    return None;
                },
            };
        }

        Some(Self {
            client
        })
    }

    pub async fn get_sites(&self) -> Option<Vec<PuzzleSite>> {
        let rows_res = self.client.query(
            "
                SELECT
                    id, name, url, css_class, variant
                FROM
                    wordle_archive.sites
                ORDER BY
                    id
            ",
            &[],
        ).await;
        let mut sites = Vec::new();
        let rows = match rows_res {
            Ok(rs) => rs,
            Err(e) => {
                error!("error querying sites: {}", e);
                return None;
            },
        };
        for row in rows {
            let site = Self::row_to_site(&row);
            sites.push(site);
        }
        Some(sites)
    }

    pub async fn get_solved_sites_for_date(&self, date: NaiveDate) -> Option<HashSet<i64>> {
        let rows_res = self.client.query(
            "
                SELECT s.id
                FROM wordle_archive.sites s
                WHERE EXISTS (
                    SELECT 1 FROM wordle_archive.puzzles p
                    WHERE p.site_id = s.id
                    AND p.puzzle_date = $1
                )
            ",
            &[&date],
        ).await;
        let mut site_ids = HashSet::new();
        let rows = match rows_res {
            Ok(rs) => rs,
            Err(e) => {
                error!("error querying solved sites: {}", e);
                return None;
            },
        };
        for row in rows {
            let site_id: i64 = row.get(0);
            site_ids.insert(site_id);
        }
        Some(site_ids)
    }

    pub async fn get_most_recent_puzzle_date(&self) -> OptionResult<NaiveDate> {
        let row_opt_res = self.client.query_opt(
            "SELECT MAX(puzzle_date) FROM wordle_archive.puzzles",
            &[],
        ).await;
        match row_opt_res {
            Ok(Some(r)) => {
                let date: NaiveDate = r.get(0);
                OptionResult::Present(date)
            },
            Ok(None) => OptionResult::Absent,
            Err(e) => {
                error!("failed to obtain maximum puzzle date: {}", e);
                OptionResult::Error
            },
        }
    }

    fn row_to_site(row: &tokio_postgres::Row) -> PuzzleSite {
        let site_id = row.get(0);
        let site_name = row.get(1);
        let site_url = row.get(2);
        let css_class = row.get(3);
        let variant = row.get(4);

        PuzzleSite {
            id: site_id,
            name: site_name,
            url: site_url,
            css_class,
            variant,
        }
    }

    fn row_to_site_and_puzzle(row: &tokio_postgres::Row) -> SiteAndPuzzle {
        let site = Self::row_to_site(row);
        let id = row.get(5);
        let date = row.get(6);
        let day_ordinal = row.get(7);
        let head = row.get(8);
        let tail = row.get(9);
        let pattern = row.get(10);
        let solution = row.get(11);
        let attempts = row.get(12);
        let raw_pattern = row.get(13);

        let puzzle = Puzzle {
            id,
            site_id: site.id,
            date,
            day_ordinal,
            head,
            tail,
            pattern,
            solution,
            attempts,
            raw_pattern,
        };
        SiteAndPuzzle {
            site,
            puzzle,
        }
    }

    pub async fn get_puzzles_on_date(&self, date: NaiveDate) -> Option<Vec<SiteAndPuzzle>> {
        let rows_res = self.client.query(
            "
                SELECT
                    site_id, site_name, site_url, site_css_class, variant, puzzle_id, puzzle_date,
                    day_ordinal, head, tail, pattern, solution, attempts, raw_pattern
                FROM
                    wordle_archive.sites_and_puzzles
                WHERE
                    puzzle_date = $1
                ORDER BY
                    site_id, day_ordinal
            ",
            &[&date],
        ).await;
        let rows = match rows_res {
            Ok(r) => r,
            Err(e) => {
                error!("failed to obtain puzzles: {}", e);
                return None;
            },
        };

        let mut puzzles = Vec::new();
        for row in rows {
            let site_and_puzzle = Self::row_to_site_and_puzzle(&row);
            puzzles.push(site_and_puzzle);
        }

        Some(puzzles)
    }

    pub async fn get_puzzle_by_id(&self, id: i64) -> OptionResult<SiteAndPuzzle> {
        let row_opt_res = self.client.query_opt(
            "
                SELECT
                    site_id, site_name, site_url, site_css_class, variant, puzzle_id, puzzle_date,
                    day_ordinal, head, tail, pattern, solution, attempts, raw_pattern
                FROM
                    wordle_archive.sites_and_puzzles
                WHERE
                    puzzle_id = $1
            ",
            &[&id],
        ).await;
        match row_opt_res {
            Ok(Some(r)) => {
                let site_and_puzzle = Self::row_to_site_and_puzzle(&r);
                OptionResult::Present(site_and_puzzle)
            },
            Ok(None) => OptionResult::Absent,
            Err(e) => {
                error!("failed to obtain puzzle by ID: {}", e);
                OptionResult::Error
            },
        }
    }

    pub async fn get_stats(&self) -> Option<Vec<Stats>> {
        let mut all_stats = Vec::new();

        // global stats
        {
            let global_row_res = self.client.query_one(
                "
                    SELECT
                        puzzles_won, puzzles_lost, average_attempts
                    FROM
                        wordle_archive.global_stats
                ",
                &[],
            ).await;
            let global_row = match global_row_res {
                Ok(r) => r,
                Err(e) => {
                    error!("failed to obtain global statistics: {}", e);
                    return None;
                },
            };

            {
                let puzzles_won = global_row.get(0);
                let puzzles_lost = global_row.get(1);
                let average_attempts = global_row.get(2);

                let stats = Stats {
                    subject: StatsSubject::Global,
                    puzzles_won,
                    puzzles_lost,
                    average_attempts,
                };
                all_stats.push(stats);
            }
        }

        // variant stats
        {
            let rows_res = self.client.query(
                "
                    SELECT
                        variant,
                        puzzles_won, puzzles_lost, average_attempts
                    FROM
                        wordle_archive.variant_stats
                    ORDER BY
                        variant
                ",
                &[],
            ).await;
            let rows = match rows_res {
                Ok(rs) => rs,
                Err(e) => {
                    error!("failed to obtain statistics: {}", e);
                    return None;
                },
            };

            for row in rows {
                let variant = row.get(0);
                let puzzles_won = row.get(1);
                let puzzles_lost = row.get(2);
                let average_attempts = row.get(3);

                let stats = Stats {
                    subject: StatsSubject::Variant(variant),
                    puzzles_won,
                    puzzles_lost,
                    average_attempts,
                };
                all_stats.push(stats);
            }
        }

        // site stats
        {
            let rows_res = self.client.query(
                "
                    SELECT
                        site_id, site_name, site_css_class,
                        puzzles_won, puzzles_lost, average_attempts
                    FROM
                        wordle_archive.site_stats
                    ORDER BY
                        site_id
                ",
                &[],
            ).await;
            let rows = match rows_res {
                Ok(rs) => rs,
                Err(e) => {
                    error!("failed to obtain statistics: {}", e);
                    return None;
                },
            };

            for row in rows {
                let site_id = row.get(0);
                let site_name = row.get(1);
                let site_css_class = row.get(2);
                let puzzles_won = row.get(3);
                let puzzles_lost = row.get(4);
                let average_attempts = row.get(5);

                let stats = Stats {
                    subject: StatsSubject::Site { id: site_id, name: site_name, css_class: site_css_class },
                    puzzles_won,
                    puzzles_lost,
                    average_attempts,
                };
                all_stats.push(stats);
            }
        }

        Some(all_stats)
    }

    pub async fn store_puzzle(&self, puzzle: &Puzzle) -> bool {
        let res = self.client.execute(
            "
                INSERT INTO
                    wordle_archive.puzzles
                    (site_id, puzzle_date, day_ordinal, head, tail, pattern, solution, attempts, raw_pattern)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ",
            &[
                &puzzle.site_id, &puzzle.date, &puzzle.day_ordinal, &puzzle.head, &puzzle.tail,
                &puzzle.pattern, &puzzle.solution, &puzzle.attempts, &puzzle.raw_pattern,
            ],
        ).await;
        if let Err(e) = res {
            error!("failed to insert puzzle: {}", e);
            false
        } else {
            true
        }
    }
}

#[async_trait]
pub(crate) trait DbMigration : Debug + Sync {
    async fn is_required(&self, db_client: &tokio_postgres::Client, current_schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error>;
    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool;
}

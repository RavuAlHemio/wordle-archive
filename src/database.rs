use chrono::NaiveDate;
use log::error;
use tokio_postgres::{self, NoTls};

use crate::config::CONFIG;
use crate::model::{Puzzle, PuzzleSite, SiteAndPuzzle};


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
            let id = row.get(0);
            let name = row.get(1);
            let url = row.get(2);
            let css_class = row.get(3);
            let variant = row.get(4);

            sites.push(PuzzleSite {
                id,
                name,
                url,
                css_class,
                variant,
            });
        }
        Some(sites)
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

    fn row_to_site_and_puzzle(row: &tokio_postgres::Row) -> SiteAndPuzzle {
        let site_id = row.get(0);
        let site_name = row.get(1);
        let site_url = row.get(2);
        let css_class = row.get(3);
        let variant = row.get(4);
        let id = row.get(5);
        let date = row.get(6);
        let day_ordinal = row.get(7);
        let head = row.get(8);
        let tail = row.get(9);
        let pattern = row.get(10);
        let solution = row.get(11);

        let site = PuzzleSite {
            id: site_id,
            name: site_name,
            url: site_url,
            css_class,
            variant,
        };
        let puzzle = Puzzle {
            id,
            site_id,
            date,
            day_ordinal,
            head,
            tail,
            pattern,
            solution,
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
                    day_ordinal, head, tail, pattern, solution
                FROM
                    wordle_archive.sites_and_puzzles
                WHERE
                    puzzle_date = $1
                ORDER BY
                    site_id
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
                    day_ordinal, head, tail, pattern, solution
                FROM
                    wordle_archive.sites_and_puzzles
                WHERE
                    puzzle_id = $1
                ORDER BY
                    site_id
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

    pub async fn store_puzzle(&self, puzzle: &Puzzle) -> bool {
        let res = self.client.execute(
            "
                INSERT INTO
                    wordle_archive.puzzles
                    (site_id, puzzle_date, day_ordinal, head, tail, pattern, solution)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)
            ",
            &[
                &puzzle.site_id, &puzzle.date, &puzzle.day_ordinal, &puzzle.head, &puzzle.tail,
                &puzzle.pattern, &puzzle.solution,
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

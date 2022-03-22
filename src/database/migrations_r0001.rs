use async_trait::async_trait;
use log::error;

use crate::database::DbMigration;


#[derive(Debug)]
pub(crate) struct MigrationR0001ToR0002;
#[async_trait]
impl DbMigration for MigrationR0001ToR0002 {
    async fn is_required(&self, db_client: &tokio_postgres::Client) -> Result<bool, tokio_postgres::Error> {
        // check for existence of schema_version table
        let rows = db_client.query(
            "
                SELECT tbl.oid
                FROM pg_catalog.pg_class tbl
                INNER JOIN pg_catalog.pg_namespace sch ON sch.oid = tbl.relnamespace
                WHERE
                    tbl.relname = 'schema_version'
                    AND tbl.relkind = 'r'
                    AND sch.nspname = 'wordle_archive'
            ",
            &[],
        ).await?;
        Ok(rows.len() == 0)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0001_to_r0002.pgsql");

        // run the migration
        match db_client.batch_execute(migration_code).await {
            Ok(_) => return true,
            Err(e) => {
                error!("failed to apply database schema migration from r0001 to r0002: {}", e);
                error!("please perform the following modifications manually on the database:\n{}", migration_code);
                return false;
            },
        };
    }
}


#[derive(Debug)]
pub(crate) struct MigrationR0002ToR0003;
#[async_trait]
impl DbMigration for MigrationR0002ToR0003 {
    async fn is_required(&self, db_client: &tokio_postgres::Client) -> Result<bool, tokio_postgres::Error> {
        // check for schema version
        let row = db_client.query_one(
            "SELECT schema_version FROM wordle_archive.schema_version",
            &[],
        ).await?;
        let version: i64 = row.get(0);
        Ok(version < 3)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        // check if column victory exists in wordle_archive.puzzles
        let table_has_victory_column = {
            let row_res = db_client.query_opt(
                "
                    SELECT 1
                    FROM pg_catalog.pg_attribute col
                    INNER JOIN pg_catalog.pg_class tbl ON tbl.oid = col.attrelid
                    INNER JOIN pg_catalog.pg_namespace sch ON sch.oid = tbl.relnamespace
                    WHERE
                        col.attname = 'victory'
                        AND tbl.relname = 'puzzles'
                        AND tbl.relkind = 'r'
                        AND sch.nspname = 'wordle_archive'
                ",
                &[],
            ).await;
            match row_res {
                Ok(Some(_row)) => true,
                Ok(None) => false,
                Err(e) => {
                    error!("failed to apply database schema migration from r0002 to r0003");
                    error!("failed to query whether table wordle_archive.puzzles has victory column: {}", e);
                    return false;
                },
            }
        };

        if !table_has_victory_column {
            // add victory column
            let add_victory_column_cmds = "
                ALTER TABLE wordle_archive.puzzles ADD COLUMN victory boolean NOT NULL DEFAULT FALSE;
                ALTER TABLE wordle_archive.puzzles ALTER COLUMN victory DROP DEFAULT;
            ";
            let add_column_res = db_client.batch_execute(add_victory_column_cmds).await;
            if let Err(e) = add_column_res {
                error!("failed to apply database schema migration from r0002 to r0003");
                error!("failed to add victory column to table wordle_archive.puzzles: {}", e);
                error!("please perform the following modifications manually on the database:\n{}", add_victory_column_cmds);
                return false;
            }
        }

        let view_has_victory_column = {
            let row_res = db_client.query_opt(
                "
                    SELECT 1
                    FROM pg_catalog.pg_attribute col
                    INNER JOIN pg_catalog.pg_class tbl ON tbl.oid = col.attrelid
                    INNER JOIN pg_catalog.pg_namespace sch ON sch.oid = tbl.relnamespace
                    WHERE
                        col.attname = 'victory'
                        AND tbl.relname = 'sites_and_puzzles'
                        AND tbl.relkind = 'v'
                        AND sch.nspname = 'wordle_archive'
                ",
                &[],
            ).await;
            match row_res {
                Ok(Some(_row)) => true,
                Ok(None) => false,
                Err(e) => {
                    error!("failed to apply database schema migration from r0002 to r0003");
                    error!("failed to query whether view wordle_archive.sites_and_puzzles has victory column: {}", e);
                    return false;
                },
            }
        };
        if !view_has_victory_column {
            // update sites_and_puzzles view
            let update_view_cmd = "
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
                        p.victory
                    FROM
                        wordle_archive.sites s
                        INNER JOIN wordle_archive.puzzles p
                            ON p.site_id = s.id
            ";
            let replace_view_res = db_client.batch_execute(update_view_cmd).await;
            if let Err(e) = replace_view_res {
                error!("failed to apply database schema migration from r0002 to r0003");
                error!("failed to update view wordle_archive.sites_and_puzzles: {}", e);
                error!("please perform the following modifications manually on the database:\n{}", update_view_cmd);
                return false;
            }
        };

        // calculate victory for existing puzzles
        let update_stmt_res = db_client.prepare(
            "UPDATE wordle_archive.puzzles SET victory=$1 WHERE id=$2",
        ).await;
        let update_stmt = match update_stmt_res {
            Ok(s) => s,
            Err(e) => {
                error!("failed to apply database schema migration from r0002 to r0003");
                error!("failed to prepare statement updating victory state for existing puzzles: {}", e);
                return false;
            },
        };

        let rows_res = db_client.query(
            "SELECT id, pattern FROM wordle_archive.puzzles",
            &[],
        ).await;
        let rows = match rows_res {
            Ok(r) => r,
            Err(e) => {
                error!("failed to apply database schema migration from r0002 to r0003");
                error!("failed to obtain patterns for existing puzzles: {}", e);
                return false;
            },
        };
        for row in rows {
            let id: i64 = row.get(0);
            let pattern: String = row.get(1);

            let last_line = pattern.split("\n").last().unwrap();
            let victory =
                last_line.contains('C')
                && !last_line.contains('W')
                && !last_line.contains('M')
            ;
            if let Err(e) = db_client.execute(&update_stmt, &[&victory, &id]).await {
                error!("failed to apply database schema migration from r0002 to r0003");
                error!("failed to set victory status to {} for puzzle {}: {}", victory, id, e);
                return false;
            }
        }

        // update schema version in database
        let update_res = db_client.execute(
            "UPDATE wordle_archive.schema_version SET schema_version=3",
            &[],
        ).await;
        if let Err(e) = update_res {
            error!("failed to apply database schema migration from r0002 to r0003");
            error!("failed to update schema version: {}", e);
            return false;
        }

        true
    }
}

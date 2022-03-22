use async_trait::async_trait;
use log::error;

use crate::database::DbMigration;
use crate::database::migration_utils;


#[derive(Debug)]
pub(crate) struct MigrationR0001ToR0002;
#[async_trait]
impl DbMigration for MigrationR0001ToR0002 {
    async fn is_required(&self, db_client: &tokio_postgres::Client) -> Result<bool, tokio_postgres::Error> {
        // check for existence of schema_version table
        let version_table_already_exists = migration_utils::table_exists(
            db_client, "wordle_archive", "schema_version",
        ).await?;
        Ok(!version_table_already_exists)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0001_to_r0002.pgsql");

        // run the migration
        match db_client.batch_execute(migration_code).await {
            Ok(_) => return true,
            Err(e) => {
                migration_utils::log_failure_error(self, &e);
                migration_utils::log_manual_commands(migration_code);
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
        migration_utils::schema_older_than(db_client, 3).await
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        // check if column victory exists in wordle_archive.puzzles
        let table_has_victory_column = {
            let vic_exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "puzzles", "victory"
            ).await;
            match vic_exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
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
                migration_utils::log_failure(self);
                error!("failed to add victory column to table wordle_archive.puzzles: {}", e);
                migration_utils::log_manual_commands(add_victory_column_cmds);
                return false;
            }
        }

        let view_has_victory_column = {
            let vic_exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "sites_and_puzzles", "victory"
            ).await;
            match vic_exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
                    error!("failed to query whether view wordle_archive.sites_and_puzzles has victory column: {}", e);
                    return false;
                },
            }
        };
        if !view_has_victory_column {
            // update sites_and_puzzles view
            let replace_view_cmd = "
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
            let replace_view_res = db_client.batch_execute(replace_view_cmd).await;
            if let Err(e) = replace_view_res {
                migration_utils::log_failure(self);
                error!("failed to update view wordle_archive.sites_and_puzzles: {}", e);
                migration_utils::log_manual_commands(replace_view_cmd);
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
                migration_utils::log_failure(self);
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
                migration_utils::log_failure(self);
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
                migration_utils::log_failure(self);
                error!("failed to set victory status to {} for puzzle {}: {}", victory, id, e);
                return false;
            }
        }

        // update schema version in database
        let update_res = migration_utils::store_schema_version(db_client, 3).await;
        if let Err(e) = update_res {
            migration_utils::log_failure(self);
            error!("failed to update schema version: {}", e);
            return false;
        }

        true
    }
}


#[derive(Debug)]
pub(crate) struct MigrationR0003ToR0004;
#[async_trait]
impl DbMigration for MigrationR0003ToR0004 {
    async fn is_required(&self, db_client: &tokio_postgres::Client) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(db_client, 4).await
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        // check if column victory exists in wordle_archive.sites_and_puzzles
        let view_has_victory_column = {
            let vic_exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "sites_and_puzzles", "victory"
            ).await;
            match vic_exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
                    error!("failed to query whether view wordle_archive.sites_and_puzzles has victory column: {}", e);
                    return false;
                },
            }
        };
        if view_has_victory_column {
            // drop sites_and_puzzles view
            let drop_view_cmd = "DROP VIEW wordle_archive.sites_and_puzzles";
            let drop_view_res = db_client.batch_execute(drop_view_cmd).await;
            if let Err(e) = drop_view_res {
                migration_utils::log_failure(self);
                error!("failed to drop view wordle_archive.sites_and_puzzles: {}", e);
                migration_utils::log_manual_commands(drop_view_cmd);
                return false;
            }
        };

        // check if column victory exists in wordle_archive.puzzles
        let table_has_victory_column = {
            let vic_exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "puzzles", "victory"
            ).await;
            match vic_exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
                    error!("failed to query whether table wordle_archive.puzzles has victory column: {}", e);
                    return false;
                },
            }
        };
        if table_has_victory_column {
            // drop victory column
            let drop_victory_column_cmds = "ALTER TABLE wordle_archive.puzzles DROP COLUMN victory;";
            let drop_column_res = db_client.batch_execute(drop_victory_column_cmds).await;
            if let Err(e) = drop_column_res {
                migration_utils::log_failure(self);
                error!("failed to drop victory column from table wordle_archive.puzzles: {}", e);
                migration_utils::log_manual_commands(drop_victory_column_cmds);
                return false;
            }
        }

        // check if column attempts exists in wordle_archive.puzzles
        let table_has_attempts_column = {
            let exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "puzzles", "attempts"
            ).await;
            match exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
                    error!("failed to query whether table wordle_archive.puzzles has attempts column: {}", e);
                    return false;
                },
            }
        };
        if !table_has_attempts_column {
            // add attempts column
            let add_column_cmds = "
                ALTER TABLE wordle_archive.puzzles ADD COLUMN attempts bigint NULL DEFAULT NULL;
                ALTER TABLE wordle_archive.puzzles ALTER COLUMN attempts DROP DEFAULT;
            ";
            let add_column_res = db_client.batch_execute(add_column_cmds).await;
            if let Err(e) = add_column_res {
                migration_utils::log_failure(self);
                error!("failed to add attempts column to table wordle_archive.puzzles: {}", e);
                migration_utils::log_manual_commands(add_column_cmds);
                return false;
            }
        }

        let view_has_attempts_column = {
            let exists_res = migration_utils::column_exists(
                db_client, "wordle_archive", "sites_and_puzzles", "attempts"
            ).await;
            match exists_res {
                Ok(existence) => existence,
                Err(e) => {
                    migration_utils::log_failure(self);
                    error!("failed to query whether view wordle_archive.sites_and_puzzles has attempts column: {}", e);
                    return false;
                },
            }
        };
        if !view_has_attempts_column {
            // replace sites_and_puzzles view
            let replace_view_cmd = "
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
                        p.attempts
                    FROM
                        wordle_archive.sites s
                        INNER JOIN wordle_archive.puzzles p
                            ON p.site_id = s.id
            ";
            let replace_view_res = db_client.batch_execute(replace_view_cmd).await;
            if let Err(e) = replace_view_res {
                migration_utils::log_failure(self);
                error!("failed to update view wordle_archive.sites_and_puzzles: {}", e);
                migration_utils::log_manual_commands(replace_view_cmd);
                return false;
            }
        };

        // calculate attempts for existing puzzles
        let update_stmt_res = db_client.prepare(
            "UPDATE wordle_archive.puzzles SET attempts=$1 WHERE id=$2",
        ).await;
        let update_stmt = match update_stmt_res {
            Ok(s) => s,
            Err(e) => {
                migration_utils::log_failure(self);
                error!("failed to prepare statement updating attempt count for existing puzzles: {}", e);
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
                migration_utils::log_failure(self);
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
            let attempts = if victory {
                Some((pattern.bytes().filter(|b| *b == b'\n').count() + 1) as i64)
            } else {
                None
            };
            if let Err(e) = db_client.execute(&update_stmt, &[&attempts, &id]).await {
                migration_utils::log_failure(self);
                error!("failed to set attempt count to {:?} for puzzle {}: {}", attempts, id, e);
                return false;
            }
        }

        // update schema version in database
        let update_res = migration_utils::store_schema_version(db_client, 4).await;
        if let Err(e) = update_res {
            migration_utils::log_failure(self);
            error!("failed to update schema version: {}", e);
            return false;
        }

        true
    }
}


#[derive(Debug)]
pub(crate) struct MigrationR0004ToR0005;
#[async_trait]
impl DbMigration for MigrationR0004ToR0005 {
    async fn is_required(&self, db_client: &tokio_postgres::Client) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(db_client, 5).await
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0004_to_r0005.pgsql");

        // run the migration
        match db_client.batch_execute(migration_code).await {
            Ok(_) => true,
            Err(e) => {
                migration_utils::log_failure_error(self, &e);
                migration_utils::log_manual_commands(migration_code);
                false
            },
        }
    }
}

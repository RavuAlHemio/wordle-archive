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

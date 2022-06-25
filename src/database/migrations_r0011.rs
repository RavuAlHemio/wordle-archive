use async_trait::async_trait;

use crate::database::DbMigration;
use crate::database::migration_utils;


#[derive(Debug)]
pub(crate) struct MigrationR0011ToR0012;
#[async_trait]
impl DbMigration for MigrationR0011ToR0012 {
    async fn is_required(&self, _db_client: &tokio_postgres::Client, schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(schema_version, 12)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0011_to_r0012.pgsql");
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

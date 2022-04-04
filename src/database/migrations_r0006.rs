use async_trait::async_trait;

use crate::database::DbMigration;
use crate::database::migration_utils;


#[derive(Debug)]
pub(crate) struct MigrationR0006ToR0007;
#[async_trait]
impl DbMigration for MigrationR0006ToR0007 {
    async fn is_required(&self, _db_client: &tokio_postgres::Client, schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(schema_version, 7)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0006_to_r0007.pgsql");
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
pub(crate) struct MigrationR0007ToR0008;
#[async_trait]
impl DbMigration for MigrationR0007ToR0008 {
    async fn is_required(&self, _db_client: &tokio_postgres::Client, schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(schema_version, 8)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0007_to_r0008.pgsql");
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
pub(crate) struct MigrationR0008ToR0009;
#[async_trait]
impl DbMigration for MigrationR0008ToR0009 {
    async fn is_required(&self, _db_client: &tokio_postgres::Client, schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(schema_version, 9)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0008_to_r0009.pgsql");
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
pub(crate) struct MigrationR0009ToR0010;
#[async_trait]
impl DbMigration for MigrationR0009ToR0010 {
    async fn is_required(&self, _db_client: &tokio_postgres::Client, schema_version: Option<i64>) -> Result<bool, tokio_postgres::Error> {
        migration_utils::schema_older_than(schema_version, 10)
    }

    async fn migrate(&self, db_client: &tokio_postgres::Client) -> bool {
        let migration_code = include_str!("../../db/migrations/r0009_to_r0010.pgsql");
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

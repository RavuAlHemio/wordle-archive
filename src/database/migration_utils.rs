use log::error;
use tokio_postgres::types::ToSql;

use crate::database::DbMigration;


async fn run_existence_query(db_client: &tokio_postgres::Client, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<bool, tokio_postgres::Error> {
    let row_opt = db_client.query_opt(query, params).await?;
    Ok(row_opt.is_some())
}

pub(crate) async fn schema_older_than(db_client: &tokio_postgres::Client, comparison_version: i64) -> Result<bool, tokio_postgres::Error> {
    let row = db_client.query_one(
        "SELECT schema_version FROM wordle_archive.schema_version",
        &[],
    ).await?;
    let version: i64 = row.get(0);
    Ok(version < comparison_version)
}

pub(crate) async fn store_schema_version(db_client: &tokio_postgres::Client, new_version: i64) -> Result<(), tokio_postgres::Error> {
    db_client.execute("UPDATE wordle_archive.schema_version SET schema_version=$1", &[&new_version]).await?;
    Ok(())
}

async fn class_exists(db_client: &tokio_postgres::Client, schema: &str, relation_kind: char, name: &str) -> Result<bool, tokio_postgres::Error> {
    let relation_kind_i8 = relation_kind as i8;
    run_existence_query(
        db_client,
        "
            SELECT tbl.oid
            FROM pg_catalog.pg_class tbl
            INNER JOIN pg_catalog.pg_namespace sch ON sch.oid = tbl.relnamespace
            WHERE
                tbl.relname = $3
                AND tbl.relkind = $2
                AND sch.nspname = $1
        ",
        &[&schema, &relation_kind_i8, &name],
    ).await
}

pub(crate) async fn table_exists(db_client: &tokio_postgres::Client, schema: &str, table: &str) -> Result<bool, tokio_postgres::Error> {
    class_exists(db_client, schema, 'r', table).await
}

#[allow(unused)]
pub(crate) async fn view_exists(db_client: &tokio_postgres::Client, schema: &str, view: &str) -> Result<bool, tokio_postgres::Error> {
    class_exists(db_client, schema, 'v', view).await
}

pub(crate) async fn column_exists(db_client: &tokio_postgres::Client, schema: &str, table_or_view: &str, column: &str) -> Result<bool, tokio_postgres::Error> {
    run_existence_query(
        db_client,
        "
            SELECT 1
            FROM pg_catalog.pg_attribute col
            INNER JOIN pg_catalog.pg_class tbl ON tbl.oid = col.attrelid
            INNER JOIN pg_catalog.pg_namespace sch ON sch.oid = tbl.relnamespace
            WHERE
                col.attname = $3
                AND tbl.relname = $2
                AND sch.nspname = $1
        ",
        &[&schema, &table_or_view, &column],
    ).await
}

pub(crate) fn log_failure<M: DbMigration>(migration: &M) {
    error!("failed to apply migration {:?}", migration);
}
pub(crate) fn log_failure_error<M: DbMigration, E: std::error::Error>(migration: &M, e: &E) {
    error!("failed to apply migration {:?}: {}", migration, e);
}
pub(crate) fn log_manual_commands(commands: &str) {
    error!("please perform the following modifications manually on the database:\n{}", commands);
}

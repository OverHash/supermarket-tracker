use error_stack::Result;
use sqlx::{Pool, Postgres};

/// Initializes the database by performing necessary migrations.
///
/// # Errors
/// If unable to migrate the database.
#[tracing::instrument(name = "initialize database", level = "debug", skip_all)]
pub async fn initialize_database(conn: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(conn).await?;

    Ok(())
}

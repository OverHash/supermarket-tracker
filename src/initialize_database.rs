use error_stack::Result;
use sqlx::{Pool, Postgres};

/// Initializes the database by performing necessary migrations.
///
/// # Errors
/// If unable to migrate the database.
pub async fn initialize_database(conn: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(conn).await?;

    Ok(())
}

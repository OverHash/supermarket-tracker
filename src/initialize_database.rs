use error_stack::Result;
use sqlx::{Pool, Postgres};

pub async fn initialize_database(conn: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(conn).await?;

    Ok(())
}

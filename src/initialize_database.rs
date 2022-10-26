use std::error::Error;

use sqlx::{Executor, PgConnection, Pool, Postgres};

pub async fn initialize_database(conn: &Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    // create the details table

    Ok(())
}

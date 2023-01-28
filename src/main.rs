use std::collections::HashSet;
use std::env;
use std::time::Duration;

use dotenvy::dotenv;
use error_stack::{IntoReport, Result, ResultExt};
use sqlx::postgres::PgPoolOptions;

use crate::error::ApplicationError;
use crate::initialize_database::initialize_database;
use crate::supermarket::{get_supermarket_type, Supermarket};

pub const CACHE_PATH: &str = "cache.json";
/// The amount of milliseconds to wait between performing iterations on the pages.
const PAGE_ITERATION_INTERVAL: Duration = Duration::from_millis(500);
/// The amount of requests to perform in parallel.
const CONCURRENT_REQUESTS: i64 = 12;

mod countdown;
mod error;
mod initialize_database;
mod new_world;
mod supermarket;

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    // ignore any error attempting to load .env file
    dotenv().ok();

    let args: Vec<_> = env::args().skip(1).collect();
    let hashed_args: HashSet<String> = args.iter().cloned().collect();

    let no_insert = hashed_args.contains("--no-insert");

    let supermarket_type =
        get_supermarket_type(&args).change_context(ApplicationError::InvalidOption {
            option: String::from("--supermarket"),
        })?;

    // connect to database
    let database_url = env::var("DATABASE_URL")
        .into_report()
        .change_context(ApplicationError::DatabaseConnectError)?;
    let connection = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .into_report()
        .change_context(ApplicationError::DatabaseConnectError)?;

    println!("Connected to database");
    initialize_database(&connection)
        .await
        .change_context(ApplicationError::DatabaseInitializeError)?;

    match supermarket_type {
        Supermarket::Countdown => countdown::run(connection, no_insert).await,
        Supermarket::NewWorld => new_world::run().await,
    }
}

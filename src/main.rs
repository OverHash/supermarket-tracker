use std::time::Duration;

use dotenvy::dotenv;
use error_stack::{Result, ResultExt};
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;
use crate::error::ApplicationError;
use crate::initialize_database::initialize_database;
use crate::supermarket::Supermarket;
use crate::telemetry::init_subscriber;

pub const CACHE_PATH: &str = "cache.json";
/// The amount of milliseconds to wait between performing iterations on the pages.
const PAGE_ITERATION_INTERVAL: Duration = Duration::from_millis(500);
/// The amount of requests to perform in parallel.
const CONCURRENT_REQUESTS: i64 = 12;

mod config;
mod countdown;
mod error;
mod initialize_database;
mod new_world;
mod supermarket;
mod telemetry;

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let subscriber = telemetry::get_tracing_subscriber(std::io::stdout);
    init_subscriber(subscriber);

    // ignore any error attempting to load .env file
    dotenv().ok();

    let config = Config::read_from_env().change_context(ApplicationError::Config)?;

    // connect to database
    tracing::debug!("Connecting to database");
    let connection = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(config.database.connection_string())
        .await
        .change_context(ApplicationError::DatabaseConnectError)?;
    tracing::debug!("Connected to database");

    initialize_database(&connection)
        .await
        .change_context(ApplicationError::DatabaseInitializeError)?;

    match config.application.supermarket {
        Supermarket::Countdown => countdown::run(connection, config.database.should_insert).await,
        Supermarket::NewWorld => new_world::run().await,
    }
}

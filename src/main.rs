use dotenvy::dotenv;
use error_stack::{Result, ResultExt};
use sqlx::postgres::PgPoolOptions;

use supermarket_tracker::{
    config::Config,
    countdown,
    error::ApplicationError,
    initialize_database::initialize_database,
    new_world,
    supermarket::Supermarket,
    telemetry::{get_tracing_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let subscriber = get_tracing_subscriber(std::io::stdout);
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

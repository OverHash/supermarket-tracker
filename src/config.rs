use std::{
    collections::HashSet,
    env,
    error::Error,
    ffi::OsStr,
    fmt::{Debug, Display},
};

use error_stack::{Report, ResultExt};
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgConnectOptions;

use crate::supermarket::{Supermarket, get_supermarket_type};

pub struct Config {
    pub application: ApplicationConfig,
    pub database: DatabaseConfig,
}

#[allow(clippy::module_name_repetitions)]
pub struct ApplicationConfig {
    /// The supermarket we are targeting to get price information for.
    pub supermarket: Supermarket,
}

#[allow(clippy::module_name_repetitions)]
pub struct DatabaseConfig {
    /// If we should insert information into the Postgres database, or if we
    /// are in read-only mode.
    pub should_insert: bool,
    /// The username to use when connect to the Postgres database.
    ///
    /// Common values include `postgres`.
    username: String,
    /// The password to use when connecting to the Postgres database.
    password: SecretString,
    /// The host to use to connect to the database. Commonly `localhost`.
    host: String,
    /// The name of the database to use. Commonly `supermarket_tracker`.
    name: String,
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ConfigError {
    /// The variable that was missing when trying to load it.
    LoadVariable { variable: String },
    InvalidOption {
        /// The invalid option the user passed.
        option: String,
    },
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadVariable { variable } => {
                write!(f, "Failed to load environment variable '{variable}'")
            }
            Self::InvalidOption { option } => write!(f, "Invalid option '{option}'"),
        }
    }
}
impl Error for ConfigError {}

impl Config {
    /// Reads configuration values from environment and arguments passed to the application.
    ///
    /// # Errors
    /// - If unable to load a section of the config.
    pub fn read_from_env() -> Result<Self, Report<ConfigError>> {
        dotenvy::dotenv().ok();
        let args = env::args().skip(1).collect::<Vec<_>>();

        Ok(Self {
            application: ApplicationConfig::read_from_env(&args)
                .attach("When loading application configuration")?,
            database: DatabaseConfig::read_from_env(&args)
                .attach("When loading database configuration")?,
        })
    }
}

impl ApplicationConfig {
    /// Reads the application configuration from environment variables passed
    /// as the primary argument.
    ///
    /// # Errors
    /// Errors if the user provides an invalid `--supermarket` option.
    fn read_from_env(args: &[String]) -> Result<Self, Report<ConfigError>> {
        let supermarket =
            get_supermarket_type(args).change_context(ConfigError::InvalidOption {
                option: "--supermarket".to_string(),
            })?;

        Ok(Self { supermarket })
    }
}

fn load_env<U>(variable: U) -> Result<String, Report<ConfigError>>
where
    U: AsRef<OsStr> + Into<String> + Clone,
{
    env::var(variable.clone()).change_context(ConfigError::LoadVariable {
        variable: variable.into(),
    })
}

impl DatabaseConfig {
    /// Reads the database configuration from environment variables passed as
    /// the primary argument.
    fn read_from_env(args: &[String]) -> Result<Self, Report<ConfigError>> {
        let user = load_env("DATABASE_USER")?;
        let password = load_env("DATABASE_PASSWORD").map(SecretString::from)?;
        let host = load_env("DATABASE_HOST")?;
        let name = load_env("DATABASE_NAME")?;

        let hashed_args = args.iter().collect::<HashSet<_>>();
        let no_insert = hashed_args.contains(&"--no-insert".to_string());

        Ok(Self {
            should_insert: !no_insert,

            username: user,
            password,
            host,
            name,
        })
    }

    /// Generates the Postgres connection options used for connecting to the database.
    #[must_use]
    pub fn connection_string(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .application_name("supermarket-tracker")
            .database(&self.name)
            .host(&self.host)
            .password(self.password.expose_secret())
            .username(&self.username)
    }
}

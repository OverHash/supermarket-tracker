use std::fmt;

use error_stack::Context;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ApplicationError {
    /// Failed to load configuration data
    Config,
    /// Failed to connect to the database
    DatabaseConnectError,
    /// Failed to initialize the database with the initial tables
    DatabaseInitializeError,
    /// Errors associated with using [`reqwest`]
    HttpError,
    /// Errors associated with retrieving the categories of products.
    CategoryRetrieval,
    /// Errors associated with retrieving the products.
    ProductRetrieval,
    /// Failed to write response cache to disk
    CacheError,
    /// Failed to insert new products into the database
    NewProductsInsertionError,
    /// Failed to insert prices of products into the database
    PriceDataInsertionError,
    /// Failed to set the location to a specified store
    SetLocation,
    /// Failed to save the store to the database
    SaveStore,
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::Config => write!(f, "Failed to load configuration"),
            ApplicationError::DatabaseConnectError => write!(f, "Failed to connect to database"),
            ApplicationError::DatabaseInitializeError => {
                write!(f, "Failed to initialize database")
            }
            ApplicationError::HttpError => {
                write!(f, "An error occurred while performing an HTTP request")
            }
            ApplicationError::CategoryRetrieval => {
                write!(f, "Failed to retrieve categories of products")
            }
            ApplicationError::ProductRetrieval => {
                write!(f, "Failed to retrieve all products")
            }
            ApplicationError::CacheError => write!(f, "Failed to write cache"),
            ApplicationError::NewProductsInsertionError => {
                write!(f, "Failed to insert new products into database")
            }
            ApplicationError::PriceDataInsertionError => {
                write!(f, "Failed to write price feed into database")
            }
            ApplicationError::SetLocation => {
                write!(f, "Failed to set location of store")
            }
            ApplicationError::SaveStore => {
                write!(f, "Failed to save the store to the database")
            }
        }
    }
}

impl Context for ApplicationError {}

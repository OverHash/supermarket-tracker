use std::fmt;

use error_stack::Context;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ApplicationError {
    /// Invalid user option provided to the binary
    InvalidOption {
        /// The option which was invalid
        option: String,
    },
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
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::InvalidOption { option } => write!(f, "Invalid option '{option}'"),
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
        }
    }
}

impl Context for ApplicationError {}

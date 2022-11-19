use std::fmt;

use error_stack::Context;

#[derive(Debug)]
pub enum ApplicationError {
    /// Invalid user option provided to the binary
    InvalidOption {
        /// The option which was invalid
        option: String,
    },
    /// Failed to connect to the database
    DatabaseConnectError {},
    /// Failed to initialize the database with the initial tables
    DatabaseInitializeError {},
    /// Errors associated with using [`reqwest`]
    HttpError {},
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::InvalidOption { option } => write!(f, "Invalid option: {}", option),
            ApplicationError::DatabaseConnectError {} => write!(f, "Failed to connect to database"),
            ApplicationError::DatabaseInitializeError {} => {
                write!(f, "Failed to initialize database")
            }
            ApplicationError::HttpError {} => {
                write!(f, "An error occurred while performing an HTTP request")
            }
        }
    }
}

impl Context for ApplicationError {}

use std::fmt;

use error_stack::Context;

#[derive(Debug)]
pub enum ApplicationError {
    InvalidOption { option: String },
    DatabaseConnectError {},
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::InvalidOption { option } => write!(f, "Invalid option: {}", option),
        }
    }
}

impl Context for ApplicationError {}

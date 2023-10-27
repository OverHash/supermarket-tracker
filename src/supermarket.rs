use std::fmt;

use error_stack::{Context, Report, ResultExt};

pub enum Supermarket {
    Countdown,
    NewWorld,
}

impl<'a> Supermarket {
    /// Retrieves all the valid values that will be parsed with [`TryFrom`]
    pub fn get_allowed_types() -> &'a [&'static str] {
        &["Countdown", "NewWorld"]
    }
}

/// A struct to represent failures to convert a given [`String`] into a [`Supermarket`].
pub struct SupermarketConversionError(String);

impl TryFrom<String> for Supermarket {
    type Error = SupermarketConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Countdown" => Ok(Supermarket::Countdown {}),
            "NewWorld" => Ok(Supermarket::NewWorld {}),
            _ => Err(SupermarketConversionError(value)),
        }
    }
}

#[derive(Debug)]
pub enum SupermarketRetrievalError {
    MissingSupermarket {},
    NotPassedSupermarket {},
    ParseError { supermarket: String },
}

impl fmt::Display for SupermarketRetrievalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupermarketRetrievalError::MissingSupermarket {} => {
                write!(f, "Failed to find '--supermarket' option passed")
            }
            SupermarketRetrievalError::NotPassedSupermarket {} => write!(
                f,
                "The '--supermarket' option was specified, but no supermarket was listed after"
            ),
            SupermarketRetrievalError::ParseError { supermarket } => {
                write!(f, "Unknown supermarket variant '{supermarket}' passed")
            }
        }
    }
}

impl Context for SupermarketRetrievalError {}

/// Attempts to retrieve the type of supermarket the user specified with the `--supermarket` option.
///
/// If the user did not provide a supermarket, [`SupermarketRetrievalError::MissingSupermarket`] is returned.
///
/// If the user passed the '--supermarket' option, but did not specify a supermarket, [`SupermarketRetrievalError::NotPassedSupermarket`] is returned.
///
/// If the user provided an invalid supermarket, [`SupermarketRetrievalError::ParseError`] is returned.
pub fn get_supermarket_type(
    args: &[String],
) -> Result<Supermarket, Report<SupermarketRetrievalError>> {
    // find the position of the supermarket
    let supermarket_type_index = args
        .iter()
        .position(|a| a == "--supermarket")
        // we want the next option passed
        .map(|pos| pos + 1)
        .ok_or(SupermarketRetrievalError::MissingSupermarket {})?;

    // retrieve and parse it
    let supermarket_type = args
        .get(supermarket_type_index)
        .ok_or(SupermarketRetrievalError::NotPassedSupermarket {})?;
    let parsed_supermarket_type = Supermarket::try_from(supermarket_type.clone())
        .map_err(|e| SupermarketRetrievalError::ParseError { supermarket: e.0 })
        .attach_printable_lazy(|| {
            format!(
                "suggestion: valid supermarket types are {}",
                Supermarket::get_allowed_types()
                    .iter()
                    .map(|s| format!("'{s}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    Ok(parsed_supermarket_type)
}

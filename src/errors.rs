use polars::error::PolarsError;
use thiserror;
#[derive(thiserror::Error, Debug)]
pub enum BorrowingError {
    #[error("Polars Function Failed: {0}")]
    PolarsError(#[from] PolarsError),
    #[error("An Error Occurred: {0}")]
    Generic(String),
}
impl From<&str> for BorrowingError {
    fn from(s: &str) -> Self {
        BorrowingError::Generic(s.to_string())
    }
}

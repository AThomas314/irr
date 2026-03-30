// Copyright (C) 2026 Ashish Thomas (Ashish T Susikaran)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use polars::error::PolarsError;
use thiserror;
use tokio::task::JoinError;
///This struct is created using the thiserror crate to help automate converting from one error type to another to make it easier to propogate errors using the try operator
/// and avoid unwraps
#[derive(thiserror::Error, Debug)]
pub enum BorrowingError {
    #[error("Polars Function Failed: {0}")]
    PolarsError(#[from] PolarsError),
    #[error("An Error Occurred: {0}")]
    Generic(String),
    #[error("Error Joining tasks:{0}")]
    JoinError(#[from] JoinError),
}
impl From<&str> for BorrowingError {
    fn from(s: &str) -> Self {
        BorrowingError::Generic(s.to_string())
    }
}

// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// A result type for our stuff, as a convenience, where the error type
// is our error type.
pub type Result<T, E = error::Error> = std::result::Result<T, E>;

pub trait OrBadParameter<T> {
  fn or_bad_parameter(self) -> Result<T>;
}

// Convert any std::result::Result<T, E> into a Result<E>. If the
// result contains an error, an Err(error::Error::InvalidParameter) is
// returned. Otherwise returns the value stored in the result.
impl<T, E: std::fmt::Display + std::fmt::Debug> OrBadParameter<T>
  for std::result::Result<T, E>
{
  fn or_bad_parameter(self) -> Result<T> {
    if let Err(e) = self {
      tracing::error!("{}", e);
      return Err(error::Error::InvalidParameter);
    }

    return Ok(self.unwrap());
  }
}

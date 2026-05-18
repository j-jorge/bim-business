// SPDX-License-Identifier: AGPL-3.0-only
use thiserror::Error;

// An error type for our stuff, saving client code from handling the
// many error types from the sub systems.
#[derive(Debug, Error)]
pub enum Error {
  #[error("Internal error")]
  Internal,
  #[error("Invalid parameter")]
  InvalidParameter,
}

impl From<tokio_postgres::Error> for Error {
  fn from(e: tokio_postgres::Error) -> Error {
    tracing::error!("tokio_postgres::Error: {}'", e);
    return Error::Internal;
  }
}

impl From<deadpool_postgres::PoolError> for Error {
  fn from(e: deadpool_postgres::PoolError) -> Error {
    tracing::error!("deadpool_postgres::PoolError: {}'", e);
    return Error::Internal;
  }
}

impl From<rand::rngs::SysError> for Error {
  fn from(e: rand::rngs::SysError) -> Error {
    tracing::error!("rand::rngs::SysError: {}'", e);
    return Error::Internal;
  }
}

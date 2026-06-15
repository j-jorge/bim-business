// SPDX-License-Identifier: AGPL-3.0-only
use thiserror::Error;

// An error type for our stuff, saving client code from handling the
// many error types from the sub systems.
#[derive(Debug, Error, PartialEq)]
pub enum Error {
  #[error("Internal error")]
  Internal,
  #[error("Invalid parameter")]
  BadParameter,
  #[error("Violation of the unique constraint")]
  UniqueViolation,
  #[error("Can't do the operation in the current state")]
  Unprocessable,
}

impl From<tokio_postgres::Error> for Error {
  fn from(e: tokio_postgres::Error) -> Error {
    tracing::error!("tokio_postgres::Error: {}.", e);

    if let Some(code) = e.code() {
      if *code == tokio_postgres::error::SqlState::UNIQUE_VIOLATION {
        return Error::UniqueViolation;
      }

      if *code == tokio_postgres::error::SqlState::FOREIGN_KEY_VIOLATION {
        return Error::Unprocessable;
      }
    }

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

impl<T> From<std::sync::PoisonError<std::sync::MutexGuard<'_, T>>> for Error {
  fn from(e: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> Error {
    tracing::error!("Mutex error: {}'", e);
    return Error::Internal;
  }
}

impl From<std::io::Error> for Error {
  fn from(e: std::io::Error) -> Error {
    tracing::error!("IO error: {}'", e);
    return Error::Internal;
  }
}

impl From<std::num::TryFromIntError> for Error {
  fn from(e: std::num::TryFromIntError) -> Error {
    tracing::error!("Integer conversion error: {}'", e);
    return Error::Internal;
  }
}

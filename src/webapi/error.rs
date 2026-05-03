// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

impl axum::response::IntoResponse for business::error::Error {
  /// Turn any error into an HTTP internal server error to be sent to the
  /// client.
  fn into_response(self) -> axum::response::Response {
    tracing::error!("Internal error: {}", &self);
    return (
      axum::http::StatusCode::INTERNAL_SERVER_ERROR,
      String::from("Internal server error"),
    )
      .into_response();
  }
}

impl From<serde_json::Error> for business::error::Error {
  fn from(e: serde_json::Error) -> business::error::Error {
    tracing::error!("Internal JSON error: {}", e);
    return business::error::Error::InvalidParameter;
  }
}

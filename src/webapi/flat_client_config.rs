// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

pub fn collect<'a>(
  output: &mut std::collections::HashMap<&'a str, serde_json::value::Value>,
  entries: &'a Vec<business::flat_client_config::Entry>,
) -> business::result::Result<()> {
  for entry in entries {
    match &entry.value {
      business::flat_client_config::Value::Int64(v) => {
        output.insert(&entry.key, serde_json::to_value(v)?)
      }
      business::flat_client_config::Value::Text(v) => {
        output.insert(&entry.key, serde_json::to_value(v)?)
      }
    };
  }

  return Ok(());
}

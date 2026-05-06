// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use base64::Engine;
use rand::Rng;
use rand::SeedableRng;

pub fn generate_token(length: u8) -> result::Result<String> {
  let mut rng: rand::rngs::StdRng =
    rand::rngs::StdRng::try_from_rng(&mut rand::rngs::SysRng)?;
  let mut bytes: Vec<u8> = vec![0; length.into()];

  rng.fill_bytes(&mut bytes);

  return Ok(base64::prelude::BASE64_STANDARD.encode(&bytes));
}

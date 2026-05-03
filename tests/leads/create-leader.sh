#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh

# No authorization header: the request should fail.
expect_get_error 401 leads/list
expect_post_error 401 leads/create

# GET not allowed
expect_get_error 405 leads/create -H "Authorization: _"

expect_post leads/create -H "Authorization: _" \
     -o "$tmp_dir"/create-1.json
token="$(jq -r . "$tmp_dir"/create-1.json)"

expect_get leads/list -H "Authorization: $token" \
     -o "$tmp_dir"/list-1.json

expect_eval_eq 1 "jq length '$tmp_dir'/list-1.json"

expect_post_error 401 leads/create -H "Authorization: _"

expect_post leads/create -H "Authorization: $token" \
     -o "$tmp_dir"/create-2.json

expect_get leads/list -H "Authorization: $token" \
     -o "$tmp_dir"/list-2.json

expect_eval_eq 2 "jq length '$tmp_dir'/list-2.json"

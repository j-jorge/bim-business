#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# No authorization header: the request should fail.
expect_get_error 401 admin/leads/list
expect_post_error 401 admin/leads/create

# GET not allowed
expect_get_error 405 admin/leads/create -H "Authorization: _"

expect_post admin/leads/create -H "Authorization: _" \
     -o "$tmp_dir"/create-1.json
token_1="$(jq -r . "$tmp_dir"/create-1.json)"

expect_get admin/leads/list -H "Authorization: $token_1" \
     -o "$tmp_dir"/list-1.json

expect_eval_eq 1 "jq length '$tmp_dir'/list-1.json"

expect_post_error 401 admin/leads/create -H "Authorization: _"

# Create a second token.
expect_post admin/leads/create -H "Authorization: $token_1" \
     -o "$tmp_dir"/create-2.json
token_2="$(jq -r . "$tmp_dir"/create-2.json)"

expect_ne "$token_1" "$token_2"

expect_get admin/leads/list -H "Authorization: $token_2" \
     -o "$tmp_dir"/list-2.json

expect_eval_eq 2 "jq length '$tmp_dir'/list-2.json"

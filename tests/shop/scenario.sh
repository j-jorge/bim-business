#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh

# List does not require authorization.
expect_get shop/list -o "$tmp_dir"/list-1.json
expect_json_eq '{}' "$tmp_dir"/list-1.json

# No authorization header: the request should fail.
expect_post_error 401 \
                  shop/update \
                  -H "Content-Type: application/json" \
                  --data '{"id-1": 11}'

# Create an administrator.
expect_post leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# With authorization header: the request should pass.
expect_post shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"id-1": 11}'
expect_get shop/list -o "$tmp_dir"/list-2.json
expect_json_eq '{"id-1": 11}' "$tmp_dir"/list-2.json

# Add more items.
expect_post shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"id-2": 22, "id-3": 33}'
expect_get shop/list -o "$tmp_dir"/list-3.json
expect_json_eq \
    '{"id-1": 11, "id-2": 22, "id-3": 33}' \
    "$tmp_dir"/list-3.json

# Modify an existing item.
expect_post shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"id-2": 202}'
expect_get shop/list -o "$tmp_dir"/list-4.json
expect_json_eq \
    '{"id-1": 11, "id-2": 202, "id-3": 33}' \
    "$tmp_dir"/list-4.json


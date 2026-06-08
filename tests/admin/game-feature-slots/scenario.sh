#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# List does not require authorization.
expect_get admin/game-feature-slots/list -o "$tmp_dir"/list-1.json
expect_json_eq '[]' "$tmp_dir"/list-1.json

# No authorization header: the request should fail.
expect_post_error 401 \
                  admin/game-feature-slots/update \
                  -H "Content-Type: application/json" \
                  --data '[{"index": 0, "coins": 10}]'

# Create an administrator.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# With authorization header: the request should pass.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 0, "coins": 10}]'
expect_get admin/game-feature-slots/list -o "$tmp_dir"/list-2.json
expect_json_eq '[{"index": 0, "coins": 10}]' "$tmp_dir"/list-2.json

# Add a more items.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 20}, {"index": 3, "coins": 30}]'
expect_get admin/game-feature-slots/list -o "$tmp_dir"/list-3.json
expect_json_eq '[
                  {"index": 0, "coins": 10},
                  {"index": 1, "coins": 20},
                  {"index": 3, "coins": 30}
                ]' \
                    "$tmp_dir"/list-3.json

# Modify an existing item.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 3, "coins": 40}]'
expect_get admin/game-feature-slots/list -o "$tmp_dir"/list-4.json
expect_json_eq '[
                  {"index": 0, "coins": 10},
                  {"index": 1, "coins": 20},
                  {"index": 3, "coins": 40}
                ]' \
                    "$tmp_dir"/list-4.json

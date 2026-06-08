#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# List does not require authorization.
expect_get admin/shop/list -o "$tmp_dir"/list-1.json
expect_json_eq '[]' "$tmp_dir"/list-1.json

# No authorization header: the request should fail.
expect_post_error 401 \
                  admin/shop/update \
                  -H "Content-Type: application/json" \
                  --data '[{"id": "id-1", "coins": 11}]'

# Create an administrator.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# With authorization header: the request should pass.
expect_post admin/shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"id": "id-1", "coins": 11}]'
expect_get admin/shop/list -o "$tmp_dir"/list-2.json
expect_json_eq '[{"id": "id-1", "coins": 11}]' "$tmp_dir"/list-2.json

# Add more items.
expect_post admin/shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"id": "id-2", "coins": 22}, {"id": "id-3", "coins": 33}]'
expect_get admin/shop/list -o "$tmp_dir"/list-3.json
expect_json_eq \
    '[
       {"id": "id-1", "coins": 11},
       {"id": "id-2", "coins": 22},
       {"id": "id-3", "coins": 33}
     ]' \
    "$tmp_dir"/list-3.json

# Modify an existing item.
expect_post admin/shop/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[{"id": "id-2", "coins": 202}]'
expect_get admin/shop/list -o "$tmp_dir"/list-4.json
expect_json_eq \
    '[
       {"id": "id-1", "coins": 11},
       {"id": "id-3", "coins": 33},
       {"id": "id-2", "coins": 202}
     ]' \
    "$tmp_dir"/list-4.json


#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# No authorization header: the requests should fail.
expect_post_error 401 \
                  admin/app-config/update \
                  -H "Content-Type: application/json" \
                  --data '[
                  {"key": "foo", "value": "1"},
                  {"key": "bar", "value": "2"}
                  ]'
expect_post_error 401 admin/app-config/erase \
                  -H "Content-Type: application/json" \
                  --data '["foo"]'
expect_post_error 401 \
                  admin/app-config/value \
                  -H "Content-Type: application/json" \
                  --data '"bar"'

# Create an administrator.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# With authorization header: the requests should pass.
expect_post admin/app-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[
                  {"key": "foo", "value": "1"},
                  {"key": "bar", "value": "2"}
                  ]'
expect_post admin/app-config/erase \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '["foo"]'
expect_post admin/app-config/value \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '"foo"' \
             -o "$tmp_dir"/foo.json
expect_post admin/app-config/value \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '"bar"' \
             -o "$tmp_dir"/bar-1.json

expect_json_eq '""' "$tmp_dir"/foo.json
expect_json_eq '"2"' "$tmp_dir"/bar-1.json

# Modify existing items.
expect_post admin/app-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[
                  {"key": "bar", "value": "22"}
                  ]'
expect_post admin/app-config/value \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '"bar"' \
             -o "$tmp_dir"/bar-2.json

expect_json_eq '"22"' "$tmp_dir"/bar-2.json

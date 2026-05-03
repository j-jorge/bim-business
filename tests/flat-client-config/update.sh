#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh

# List does not require authorization.
expect_get flat-client-config/list -o "$tmp_dir"/list-1.json
expect_eval_eq 0 "jq length '$tmp_dir'/list-1.json"

# No authorization header: the request should fail.
expect_post_error 401 \
                  flat-client-config/update \
                  -H "Content-Type: application/json" \
                  --data '{"foo": 1}'

# Create an administrator.
expect_post leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# With authorization header: the request should pass.
expect_post flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"a": 123, "b": "bbu"}'
expect_get flat-client-config/list -o "$tmp_dir"/list-2.json
expect_json_eq '{"a": 123, "b": "bbu"}' "$tmp_dir"/list-2.json

# Add more entries
expect_post flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"c": -24, "d": "ddd"}'
expect_get flat-client-config/list -o "$tmp_dir"/list-3.json
expect_json_eq '{"a": 123, "b": "bbu", "c": -24, "d": "ddd"}' \
               "$tmp_dir"/list-3.json

# Modify existing items.
expect_post flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"b": "bbb", "c": "ccc", "d": -42}'
expect_get flat-client-config/list -o "$tmp_dir"/list-4.json
expect_json_eq '{"a": 123, "b": "bbb", "c": "ccc", "d": -42}' \
               "$tmp_dir"/list-4.json

# Bad JSON values should be refused.
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data 'null'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '[]'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '32'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '84.19'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '"nope"'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"ok": 0, "fail": null}'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"ok": 0, "fail": []}'
expect_post_error 400 flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"ok": 0, "fail": 0.1}'

# Get the config one last time, it should not have changed.
expect_get flat-client-config/list -o "$tmp_dir"/list-5.json
expect_json_eq '{"a": 123, "b": "bbb", "c": "ccc", "d": -42}' \
               "$tmp_dir"/list-5.json

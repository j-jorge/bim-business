#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
token="$(jq -r . "$tmp_dir"/lead.json)"

# Add some values in the config.
expect_post admin/flat-client-config/update \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '{"a": 123, "b": "bbb", "c": 42, "d": "ddd"}'
expect_get admin/flat-client-config/list -o "$tmp_dir"/list-2.json
expect_json_eq '{"a": 123, "b": "bbb", "c": 42, "d": "ddd"}' \
               "$tmp_dir"/list-2.json

# Then erase some entries.
expect_post admin/flat-client-config/erase \
            -H "Authorization: $token" \
            -H "Content-Type: application/json" \
            --data '["b", "c"]'
expect_get admin/flat-client-config/list -o "$tmp_dir"/list-3.json
expect_json_eq '{"a": 123, "d": "ddd"}' \
               "$tmp_dir"/list-3.json

#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator to test forced coins transactions.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

# First client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "abc"}' \
            -o "$tmp_dir"/authenticate-1.json
session_token_1="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id_1="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

# Second client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-2.json
session_token_2="$(jq -r .session_token "$tmp_dir"/authenticate-2.json)"
user_id_2="$(jq -r .user_id "$tmp_dir"/authenticate-2.json)"

# No coins by default.
expect_post client/wallet \
            --header "Authorization: $session_token_1" \
            -o "$tmp_dir"/coins-1.json
expect_post client/wallet \
            --header "Authorization: $session_token_2" \
            -o "$tmp_dir"/coins-2.json

expect_json_eq '{"coins": 0}' "$tmp_dir"/coins-1.json
expect_json_eq '{"coins": 0}' "$tmp_dir"/coins-2.json

# Coin transaction forced by an administrator.
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id_1"',
                      "amount": 11,
                      "reason": "test"
                    }'
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id_2"',
                      "amount": 22,
                      "reason": "test"
                    }'

expect_post client/wallet \
            --header "Authorization: $session_token_1" \
            -o "$tmp_dir"/coins-3.json
expect_post client/wallet \
            --header "Authorization: $session_token_2" \
            -o "$tmp_dir"/coins-4.json

expect_json_eq '{"coins": 11}' "$tmp_dir"/coins-3.json
expect_json_eq '{"coins": 22}' "$tmp_dir"/coins-4.json

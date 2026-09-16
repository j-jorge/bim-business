#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh
start_server

# Create an administrator.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "users.max_nickname_length",
                       "value": "5"
                    }]'

# Authenticate the user.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-1.json
session_token_1="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id_1="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

# Remember its default nickname, for the next tests.
expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_1]" \
            -o "$tmp_dir"/profile-1.json
nickname_1="$(jq -r .[0].nickname "$tmp_dir"/profile-1.json)"

# Try to set a too-long nickname.
expect_post client/account/update-nickname \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "foobar"}' \
            -o "$tmp_dir"/update-nickname-1.json

expect_json_eq '{"status": 1}' "$tmp_dir"/update-nickname-1.json

expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_1]" \
            -o "$tmp_dir"/profile-2.json

expect_json_eq \
    '[
       {"nickname": "'"$nickname_1"'", "user_id": '"$user_id_1"'}
     ]' \
         "$tmp_dir"/profile-2.json

# Now set a nickname fulfilling the constraints.
expect_post client/account/update-nickname \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "foo"}' \
            -o "$tmp_dir"/update-nickname-2.json

expect_json_eq '{"status": 0}' "$tmp_dir"/update-nickname-2.json

expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_1]" \
            -o "$tmp_dir"/profile-3.json

expect_json_eq \
    '[
       {"nickname": "foo", "user_id": '"$user_id_1"'}
     ]' \
         "$tmp_dir"/profile-3.json

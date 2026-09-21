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
            --data '[
                      {
                        "key": "users.nickname_length.min",
                        "value": "3"
                      },
                      {
                        "key": "users.nickname_length.max",
                        "value": "5"
                      },
                      {
                        "key": "users.nickname_change_cooldown.minutes",
                        "value": "1000"
                      }
                    ]'

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
expect_post_error 400 client/account/update-nickname \
                  --header "Authorization: $session_token_1" \
                  --header "Content-Type: application/json" \
                  --data '{"nickname": "foobar"}'

# Try to set a too-short nickname.
expect_post_error 400 client/account/update-nickname \
                  --header "Authorization: $session_token_1" \
                  --header "Content-Type: application/json" \
                  --data '{"nickname": "fr"}'

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
            --data '{"nickname": " foo  "}' \
            -o "$tmp_dir"/update-nickname-1.json

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

# Second update is rejected because of the cooldown.
expect_post_error 422 client/account/update-nickname \
                  --header "Authorization: $session_token_1" \
                  --header "Content-Type: application/json" \
                  --data '{"nickname": "bar"}'

expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_1]" \
            -o "$tmp_dir"/profile-4.json

expect_json_eq \
    '[
       {"nickname": "foo", "user_id": '"$user_id_1"'}
     ]' \
         "$tmp_dir"/profile-4.json

# Remove the cooldown to allow a nickname change.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[
                      {
                        "key": "users.nickname_change_cooldown.minutes",
                        "value": "0"
                      }
                    ]'

expect_post client/account/update-nickname \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "bar"}' \
            -o "$tmp_dir"/update-nickname-2.json

expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_1]" \
            -o "$tmp_dir"/profile-5.json

expect_json_eq \
    '[
       {"nickname": "bar", "user_id": '"$user_id_1"'}
     ]' \
         "$tmp_dir"/profile-5.json

expect_post client/me \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            -o "$tmp_dir"/me.json

# The response do not have the same precision in the fractional part.
date_in_response="$(jq -c --raw-output .nickname_change_allowed_date \
                       "$tmp_dir"/update-nickname-2.json \
                       | sed 's/\.[0-9]\+Z/Z/')"
date_in_me="$(jq -c --raw-output .nickname_change_allowed_date \
                       "$tmp_dir"/me.json \
                       | sed 's/\.[0-9]\+Z/Z/')"

expect_eq "$date_in_response" "$date_in_me"

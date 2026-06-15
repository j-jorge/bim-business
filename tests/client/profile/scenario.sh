#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator, it is required to override a nickname.
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

expect_post client/account/update-nickname \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "client-1"}'

# Second client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-2.json
session_token_2="$(jq -r .session_token "$tmp_dir"/authenticate-2.json)"
user_id_2="$(jq -r .user_id "$tmp_dir"/authenticate-2.json)"

expect_post client/account/update-nickname \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "foobar"}'

# Nicknames when first user asks.
expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_2, $user_id_1]" \
            -o "$tmp_dir"/profile-1.json

expect_json_eq \
    '[
       {"nickname": "client-1", "user_id": '"$user_id_1"'},
       {"nickname": "foobar", "user_id": '"$user_id_2"'}
     ]' \
         "$tmp_dir"/profile-1.json

# Nicknames when second user asks.
expect_post client/profile \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data "[$user_id_2, $user_id_1]" \
            -o "$tmp_dir"/profile-2.json

expect_json_eq \
    '[
       {"nickname": "client-1", "user_id": '"$user_id_1"'},
       {"nickname": "foobar", "user_id": '"$user_id_2"'}
     ]' \
         "$tmp_dir"/profile-2.json

# Force 2nd user's nickname.
expect_post admin/users/override-nickname \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{"user_id": '"$user_id_2"', "nickname": "client-2"}'

# Nicknames when first user asks: sees overridden nickname of second user.
expect_post client/profile \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data "[$user_id_2, $user_id_1]" \
            -o "$tmp_dir"/profile-3.json

expect_json_eq \
    '[
       {"nickname": "client-1", "user_id": '"$user_id_1"'},
       {"nickname": "client-2", "user_id": '"$user_id_2"'}
     ]' \
         "$tmp_dir"/profile-3.json

# Nicknames when second user asks: sees its own nickname.
expect_post client/profile \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data "[$user_id_2, $user_id_1]" \
            -o "$tmp_dir"/profile-4.json

expect_json_eq \
    '[
       {"nickname": "client-1", "user_id": '"$user_id_1"'},
       {"nickname": "foobar", "user_id": '"$user_id_2"'}
     ]' \
         "$tmp_dir"/profile-4.json

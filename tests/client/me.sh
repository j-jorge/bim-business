#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh
start_server

# Create an administrator, it is required to override a nickname.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

#-------------------------------------------------------------------------------
# Set up

# The client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "abc"}' \
            -o "$tmp_dir"/authenticate-1.json
session_token="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

expect_post client/account/update-nickname \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"nickname": "the-nickname"}'

# Give some coins to the user.
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id"',
                      "amount": 1234,
                      "reason": "test"
                    }'

# Populate the server with some game features.
expect_post admin/game-features/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[
                      {"name": "feat-1", "coins": 0},
                      {"name": "feat-2", "coins": 0},
                      {"name": "feat-3", "coins": 0},
                      {"name": "feat-4", "coins": 0}
                    ]'

# Populate the server with some game feature slots.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 0}]'

expect_post client/game-feature/buy-slot \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"slot_index": 1}'

expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-1"}'

expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-3"}'

expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-4"}'

expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 1, "feature": "feat-3"}
                      ]
                    }'

#-------------------------------------------------------------------------------
# Actual tests

expect_post client/me \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            -o "$tmp_dir"/me.json

expect_json_eq \
    '{
       "user_id": '"$user_id"',
       "nickname": "the-nickname",
       "coins": 1234,
       "feature_slots":
       [
         {"slot_index": 0, "feature": null},
         {"slot_index": 1, "feature": "feat-3"}
       ],
       "available_features": [ "feat-1", "feat-3", "feat-4" ],
       "arena_stats":
       {
         "victories": 0,
         "defeats": 0,
         "draws": 0
       }
     }' \
         "$tmp_dir"/me.json

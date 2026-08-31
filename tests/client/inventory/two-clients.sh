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

#-------------------------------------------------------------------------------
# Set up

# Populate the server with some game features.
expect_post admin/game-features/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[
                      {"name": "feat-1", "coins": 1000},
                      {"name": "feat-2", "coins": 22},
                      {"name": "feat-3", "coins": 33},
                      {"name": "feat-4", "coins": 44}
                    ]'

# Populate the server with some game feature slots.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 100}]'

# Give some coins to the users such that they can buy stuff.
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id_1"',
                      "amount": 1000,
                      "reason": "test"
                    }'
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id_2"',
                      "amount": 1000,
                      "reason": "test"
                    }'

#-------------------------------------------------------------------------------
# Actual tests

# Buying features.
expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-1"}'
expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-2"}'

expect_post client/game-feature/buy-slot \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data '{"slot_index": 1}'

expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token_1" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-1"}
                      ]
                    }'
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token_2" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 1, "feature": "feat-2"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token_1" \
            -o "$tmp_dir"/inventory-1.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": "feat-1"}],
                  "available_features": ["feat-1"]
                }' \
               "$tmp_dir"/inventory-1.json

expect_post client/game-feature/inventory \
            --header "Authorization: $session_token_2" \
            -o "$tmp_dir"/inventory-2.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": null},
                    {"slot_index": 1, "feature": "feat-2"}
                  ],
                  "available_features": ["feat-2"]
                }' \
               "$tmp_dir"/inventory-2.json

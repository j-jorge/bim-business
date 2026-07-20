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
session_token="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

#-------------------------------------------------------------------------------
# Set up

# Populate the server with some game features.
expect_post admin/game-features/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[
                      {"name": "feat-1", "coins": 11},
                      {"name": "feat-2", "coins": 22},
                      {"name": "feat-3", "coins": 33},
                      {"name": "feat-4", "coins": 44}
                    ]'

# Populate the server with some game feature slots.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 100}]'

#-------------------------------------------------------------------------------
# Actual tests

# Legacy inventory transfer is disabled, the inventory should not change.
expect_post client/transfer-legacy-inventory \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "coins": 123,
                      "game_features": ["feat-2", "feat-4", "feat-3"],
                      "slots": [0, 1],
                      "game_feature_selection":
                      [
                        {"slot_index": 0, "feature": "feat-2"},
                        {"slot_index": 1, "feature": "feat-4"}
                      ],
                      "arena_stats":
                      {
                        "game_count": 200,
                        "victory_count": 100,
                        "defeat_count": 50
                      }
                    }' \
            -o "$tmp_dir"/transfer-1.json
expect_json_eq '{"transfer_state": 0}' "$tmp_dir"/transfer-1.json

expect_db "select * from user_arena_statistics;" "$tmp_dir"/db-user-stats-1.txt
expect_true grep --quiet '(0 rows)' "$tmp_dir"/db-user-stats-1.txt

expect_post client/wallet \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/wallet-1.json
expect_json_eq '{"coins": 0}' "$tmp_dir"/wallet-1.json

expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-1.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": null}
                  ],
                  "available_features": []
                }' \
               "$tmp_dir"/inventory-1.json

# Enable legacy inventory transfer.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "legacy.enable_transfer",
                       "value": "true"
                    }]'

# Now the transfer should work.
expect_post client/transfer-legacy-inventory \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "coins": 123,
                      "game_features": ["feat-2", "feat-4", "feat-3"],
                      "slots": [0, 1],
                      "game_feature_selection":
                      [
                        {"slot_index": 0, "feature": "feat-2"},
                        {"slot_index": 1, "feature": "feat-4"}
                      ],
                      "arena_stats":
                      {
                        "game_count": 200,
                        "victory_count": 100,
                        "defeat_count": 50
                      }
                    }' \
            -o "$tmp_dir"/transfer-2.json
expect_json_eq '{"transfer_state": 1}' "$tmp_dir"/transfer-2.json

expect_db "select * from user_arena_statistics;" "$tmp_dir"/db-user-stats-2.txt
expect_true grep --quiet \
            '^user_id *| *'"$user_id"'$' \
            "$tmp_dir"/db-user-stats-2.txt
expect_true grep --quiet \
            '^game_count *| *200$' \
            "$tmp_dir"/db-user-stats-2.txt
expect_true grep --quiet \
            '^victories *| *100$' \
            "$tmp_dir"/db-user-stats-2.txt
expect_true grep --quiet \
            '^defeats *| *50$' \
            "$tmp_dir"/db-user-stats-2.txt
expect_eval_eq 1 "grep --count '^user_id' '$tmp_dir'/db-user-stats-2.txt"

expect_post client/wallet \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/wallet-2.json
expect_json_eq '{"coins": 123}' "$tmp_dir"/wallet-2.json

expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-2.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-2"},
                    {"slot_index": 1, "feature": "feat-4"}
                  ],
                  "available_features": ["feat-2", "feat-3", "feat-4"]
                }' \
               "$tmp_dir"/inventory-2.json

# Legacy inventory transfer should work only once.
expect_post client/transfer-legacy-inventory \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "coins": 222,
                      "game_features": ["feat-1", "feat-4", "feat-3"],
                      "slots": [0, 1],
                      "game_feature_selection":
                      [
                        {"slot_index": 0, "feature": "feat-3"},
                        {"slot_index": 1, "feature": "feat-1"}
                      ],
                      "arena_stats":
                      {
                        "game_count": 300,
                        "victory_count": 200,
                        "defeat_count": 100
                      }
                    }' \
            -o "$tmp_dir"/transfer-3.json
expect_json_eq '{"transfer_state": 2}' "$tmp_dir"/transfer-3.json

expect_post client/wallet \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/wallet-3.json
expect_json_eq '{"coins": 123}' "$tmp_dir"/wallet-3.json

expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-3.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-2"},
                    {"slot_index": 1, "feature": "feat-4"}
                  ],
                  "available_features": ["feat-2", "feat-3", "feat-4"]
                }' \
               "$tmp_dir"/inventory-3.json

expect_db "select * from user_arena_statistics;" "$tmp_dir"/db-user-stats-3.txt
expect_true grep --quiet \
            '^user_id *| *'"$user_id"'$' \
            "$tmp_dir"/db-user-stats-3.txt
expect_true grep --quiet \
            '^game_count *| *200$' \
            "$tmp_dir"/db-user-stats-3.txt
expect_true grep --quiet \
            '^victories *| *100$' \
            "$tmp_dir"/db-user-stats-3.txt
expect_true grep --quiet \
            '^defeats *| *50$' \
            "$tmp_dir"/db-user-stats-3.txt
expect_eval_eq 1 "grep --count '^user_id' '$tmp_dir'/db-user-stats-3.txt"

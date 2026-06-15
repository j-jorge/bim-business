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
                      {"name": "feat-1", "coins": 1000},
                      {"name": "feat-2", "coins": 22},
                      {"name": "feat-3", "coins": 33},
                      {"name": "feat-4", "coins": 44}
                    ]'

# Populate the server with some game feature slots.
expect_post admin/game-feature-slots/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 100}, {"index": 2, "coins": 10}]'

# Give some coins to the user such that they can buy stuff.
expect_post admin/users/coins-transaction \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "user_id": '"$user_id"',
                      "amount": 100,
                      "reason": "test"
                    }'

#-------------------------------------------------------------------------------
# Actual tests

# The inventory should be empty. Only the first game feature slot is
# available.
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-init.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": null}],
                  "available_features": []
                }' \
               "$tmp_dir"/inventory-init.json

# Buying non-existent feature -> fail.
expect_post_error 422 client/game-feature/buy-feature \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"feature_name": "nope"}'

# Buying features -> pass.
expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-2"}'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-1.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": null}],
                  "available_features": ["feat-2"]
                }' \
               "$tmp_dir"/inventory-1.json

expect_post client/game-feature/buy-feature \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"feature_name": "feat-3"}'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-2.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": null}],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-2.json

# Buying expensive features -> fail.
expect_post_error 422 client/game-feature/buy-feature \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"feature_name": "feat-1"}'

# Buying owned feature -> fail.
expect_post_error 409 client/game-feature/buy-feature \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"feature_name": "feat-2"}'

# Assigning non-owned feature to owned slot -> fail.
expect_post_error 422 client/game-feature/assign-slots \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{
                            "selection":
                            [
                              {"slot_index": 0, "feature": "feat-1"}
                            ]
                          }'

# Assigning non-owned feature to non-owned slot -> fail.
expect_post_error 422 client/game-feature/assign-slots \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{
                            "selection":
                            [
                              {"slot_index": 1, "feature": "feat-1"}
                            ]
                          }'

# Assigning owned feature to non-owned slot -> fail.
expect_post_error 422 client/game-feature/assign-slots \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{
                            "selection":
                            [
                              {"slot_index": 1, "feature": "feat-2"}
                            ]
                          }'

# Assigning owned feature to owned slot -> pass.
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-2"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-3.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": "feat-2"}],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-3.json

# Clear non-owned slot -> pass.
expect_post client/game-feature/clear-slot \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '1'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-4.json
expect_json_eq '{
                  "slots": [{"slot_index": 0, "feature": "feat-2"}],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-4.json

# Buying expensive slot -> fail.
expect_post_error 422 client/game-feature/buy-slot \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"slot_index": 1}'

# Buying non-owned slot -> pass.
expect_post client/game-feature/buy-slot \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{"slot_index": 2}'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-5.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-2"},
                    {"slot_index": 2, "feature": null}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-5.json

# Buying implicit slot -> fail.
expect_post_error 422 client/game-feature/buy-slot \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"slot_index": 0}'

# Buying owned slot -> fail.
expect_post_error 409 client/game-feature/buy-slot \
                  --header "Authorization: $session_token" \
                  --header "Content-Type: application/json" \
                  --data '{"slot_index": 2}'

# Clear owned slot -> pass.
expect_post client/game-feature/clear-slot \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '0'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-6.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": null},
                    {"slot_index": 2, "feature": null}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-6.json

# Clear empty slot -> pass.
expect_post client/game-feature/clear-slot \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '0'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-7.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": null},
                    {"slot_index": 2, "feature": null}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-7.json

# Assigning two features -> pass.
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-2"},
                        {"slot_index": 2, "feature": "feat-3"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-8.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-2"},
                    {"slot_index": 2, "feature": "feat-3"}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-8.json

# Assigning two features again -> pass.
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-3"},
                        {"slot_index": 2, "feature": "feat-2"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-9.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-3"},
                    {"slot_index": 2, "feature": "feat-2"}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-9.json

# Reassigning same feature to same slot -> pass.
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-3"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-10.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-3"},
                    {"slot_index": 2, "feature": "feat-2"}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-10.json

# Assigning same feature in two slots -> pass.
expect_post client/game-feature/assign-slots \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "selection":
                      [
                        {"slot_index": 0, "feature": "feat-2"},
                        {"slot_index": 2, "feature": "feat-2"}
                      ]
                    }'
expect_post client/game-feature/inventory \
            --header "Authorization: $session_token" \
            -o "$tmp_dir"/inventory-11.json
expect_json_eq '{
                  "slots":
                  [
                    {"slot_index": 0, "feature": "feat-2"},
                    {"slot_index": 2, "feature": "feat-2"}
                  ],
                  "available_features": ["feat-2", "feat-3"]
                }' \
               "$tmp_dir"/inventory-11.json

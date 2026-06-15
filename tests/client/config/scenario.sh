#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator such that we can populate the server with
# some data.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
lead_token="$(jq -r . "$tmp_dir"/lead.json)"

# A couple of properties.
expect_post admin/flat-client-config/update \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '{"prop-1": 123, "prop-2": "bbb"}' \
            -o /dev/null

# Some game features.
expect_post admin/game-features/update \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '[
                      {"name": "feature-1", "coins": 11},
                      {"name": "feature-2", "coins": 22}
                    ]' \
            -o /dev/null

expect_post admin/shop/update \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '[
                      {"id": "product-1", "coins": 100},
                      {"id": "product-2", "coins": 200}
                    ]' \
            -o /dev/null

# Game feature slots
expect_post admin/game-feature-slots/update \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '[{"index": 1, "coins": 20}, {"index": 3, "coins": 30}]'

# Create a token for a new game server.
expect_post admin/game-servers/register \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '{"id": "server-1", "description": "Test server 1."}' \
            -o "$tmp_dir"/"game-server-1.json"
gs_token_1="$(jq -r . "$tmp_dir"/game-server-1.json)"

expect_post admin/game-servers/register \
            -H "Authorization: $lead_token" \
            -H "Content-Type: application/json" \
            --data '{"id": "server-2", "description": "Test server 2."}' \
            -o "$tmp_dir"/"game-server-2.json"
gs_token_2="$(jq -r . "$tmp_dir"/game-server-2.json)"

# The game servers must be alive to appear in the configuration.
expect_post gs/hello \
            -H "Content-Type: application/json" \
            -H "Authorization: $gs_token_1" \
            --data \
            '{
               "host": "1.1.1.1:1111",
               "version": 1,
               "protocol_version": 11
             }' \
            -o /dev/null
expect_post gs/hello \
            -H "Content-Type: application/json" \
            -H "Authorization: $gs_token_2" \
            --data \
            '{
               "host": "2.2.2.2:2222",
               "version": 2,
               "protocol_version": 22
             }' \
            -o /dev/null

# Now confirm that the client will receive the expected config.
expect_post client/config \
            -H "Content-Type: application/json" \
            --data \
            '{
               "game_server_protocol_version": 11
             }' \
             -o "$tmp_dir"/config-1.json
expect_json_eq \
    '{
       "misc": {"prop-1": 123, "prop-2": "bbb"},
       "game_feature_slots": [
         {"index": 1, "coins": 20},
         {"index": 3, "coins": 30}
       ],
       "game_features": [
         {"name": "feature-1", "coins": 11},
         {"name": "feature-2", "coins": 22}
       ],
       "shop": [
         {"id": "product-1", "coins": 100},
         {"id": "product-2", "coins": 200}
       ],
       "game_servers": [ "1.1.1.1:1111" ]
     }' \
         "$tmp_dir"/config-1.json

expect_post client/config \
            -H "Content-Type: application/json" \
            --data \
            '{
               "game_server_protocol_version": 22
             }' \
             -o "$tmp_dir"/config-2.json
expect_json_eq \
    '{
       "misc": {"prop-1": 123, "prop-2": "bbb"},
       "game_feature_slots": [
         {"index": 1, "coins": 20},
         {"index": 3, "coins": 30}
       ],
       "game_features": [
         {"name": "feature-1", "coins": 11},
         {"name": "feature-2", "coins": 22}
       ],
       "shop": [
         {"id": "product-1", "coins": 100},
         {"id": "product-2", "coins": 200}
       ],
       "game_servers": [ "2.2.2.2:2222" ]
     }' \
         "$tmp_dir"/config-2.json

expect_post client/config \
            -H "Content-Type: application/json" \
            --data \
            '{
               "game_server_protocol_version": 42
             }' \
             -o "$tmp_dir"/config-3.json
expect_json_eq \
    '{
       "misc": {"prop-1": 123, "prop-2": "bbb"},
       "game_feature_slots": [
         {"index": 1, "coins": 20},
         {"index": 3, "coins": 30}
       ],
       "game_features": [
         {"name": "feature-1", "coins": 11},
         {"name": "feature-2", "coins": 22}
       ],
       "shop": [
         {"id": "product-1", "coins": 100},
         {"id": "product-2", "coins": 200}
       ],
       "game_servers": []
     }' \
         "$tmp_dir"/config-3.json

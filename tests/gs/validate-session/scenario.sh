#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create the administrator.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

# Register a game server.
expect_post admin/game-servers/register \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"name": "gs", "description": "..."}' \
            -o "$tmp_dir"/"gs-1.json"
gs_token="$(jq -r .token "$tmp_dir"/gs-1.json)"

# Authenticate a client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "abc"}' \
            -o "$tmp_dir"/authenticate-1.json
session_token_1="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id_1="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

# Set the session validity duration as zero such that the next created
# session is immediately invalid.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "sessions.validity.minutes",
                       "value": "0"
                    }]'

# Authenticate another client, as said above with a session that
# becomes invalid immediately.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-2.json
session_token_2="$(jq -r .session_token "$tmp_dir"/authenticate-2.json)"

# The game server checks the session of the first client. It should be valid.
expect_post gs/user-id \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "sessions":
                      [
                        "'"$session_token_2"'",
                        "'"$session_token_1"'",
                        "this-is-not-a-session"
                      ],
                      "tokens":
                      [
                        11,
                        22,
                        33
                      ]
                    }' \
                        -o "$tmp_dir"/gs-user-id-1.json
expect_json_eq '{
                  "user_ids":
                  [
                    null,
                    '"$user_id_1"',
                    null
                  ],
                  "tokens":
                  [
                    11,
                    22,
                    33
                  ]
                }' \
                    "$tmp_dir"/gs-user-id-1.json


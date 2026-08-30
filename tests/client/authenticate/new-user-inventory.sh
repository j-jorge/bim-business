#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator, it is required to update the app config.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

# Long sessions for the first client such that we have time check that
# multiple authentications from the same client produce the same
# token.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "sessions.validity.minutes",
                       "value": "10"
                    }]'
# Remove expired sessions as frequently as possible for this test.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "sessions.clean_up_interval.minutes",
                       "value": "0"
                    }]'

# Authenticate the client for the first time ever.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "abc"}' \
            -o "$tmp_dir"/authenticate-1.json
session_token="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

#-------------------------------------------------------------------------------
# Check the default inventory of a new player.

expect_post client/me \
            --header "Authorization: $session_token" \
            --header "Content-Type: application/json" \
            -o "$tmp_dir"/me.json

expect_json_eq \
    '{
       "user_id": '"$user_id"',
       "nickname": "user_'"$user_id"'",
       "coins": 0,
       "feature_slots":
       [
         {"slot_index": 0, "feature": null}
       ],
       "available_features": [],
       "arena_stats":
       {
         "victories": 0,
         "defeats": 0,
         "draws": 0
       }
     }' \
         "$tmp_dir"/me.json


#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

# Create an administrator, it is required to create game server
# tokens.
expect_post admin/leads/create -H "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

# Create a token for a new game server.
expect_post admin/game-servers/register \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"id": "valid-server", "description": "Some server."}' \
            -o "$tmp_dir"/"register-1.json"
gs_token="$(jq -r . "$tmp_dir"/register-1.json)"

expect_ne "" "$gs_token"

# Registering the same server ID twice should fail.
expect_post_error 500 admin/game-servers/register \
                  -H "Authorization: $admin_token" \
                  -H "Content-Type: application/json" \
                  --data '{"id": "valid-server", "description": "Some server."}'

# Create a token for another game server.
expect_post admin/game-servers/register \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"id": "other-server", "description": "Other server."}' \
            -o "$tmp_dir"/"register-2.json"
gs_token_2="$(jq -r . "$tmp_dir"/register-2.json)"

# A bunch of invalid IDs.
expect_post_error 400 admin/game-servers/register \
                  -H "Authorization: $admin_token" \
                  -H "Content-Type: application/json" \
                  --data '{"id": "some server", "description": "..."}'
expect_post_error 400 admin/game-servers/register \
                  -H "Authorization: $admin_token" \
                  -H "Content-Type: application/json" \
                  --data '{"id": "some%server", "description": "..."}'
expect_post_error 400 admin/game-servers/register \
                  -H "Authorization: $admin_token" \
                  -H "Content-Type: application/json" \
                  --data '{"id": "s¤me server", "description": "..."}'

# Get the list of registered servers.
expect_get admin/game-servers/list \
           -H "Authorization: $admin_token" \
           -o "$tmp_dir"/list-1.json
# Can't get the list if we are no admin.
expect_get_error 401 admin/game-servers/list \
           -H "Authorization: foobar"
expect_get_error 401 admin/game-servers/list

sed 's/\(registration_date":"\)[^"]\+/\1placeholder/g' \
    -i "$tmp_dir"/list-1.json
expect_json_eq \
    '{
        "valid-server":
        {
          "online": false,
          "description": "Some server.",
          "token": "'"$gs_token"'",
          "last_seen": "1970-01-01T00:00:00+00:00",
          "registration_date": "placeholder"
        },
        "other-server":
        {
          "online": false,
          "description": "Other server.",
          "token": "'"$gs_token_2"'",
          "last_seen": "1970-01-01T00:00:00+00:00",
          "registration_date": "placeholder"
        }
     }' \
         "$tmp_dir"/list-1.json

# One game server tells us that it is alive.
expect_post admin/game-servers/keep-alive \
            -H "Content-Type: application/json" \
            --data \
            '{
               "token": "'"$gs_token_2"'",
               "host": "1.2.3.4:1234",
               "version": 42,
               "protocol_version": 24
             }' \
             -o "$tmp_dir"/keep-alive-1.json
sed 's/\(callback_delay_seconds":\)[0-9]\+/\1"placeholder"/' \
    -i "$tmp_dir"/keep-alive-1.json
expect_json_eq '{"callback_delay_seconds":"placeholder"}' \
               "$tmp_dir"/keep-alive-1.json

# Same but with a domain instead of an IP for the host.
expect_post admin/game-servers/keep-alive \
            -H "Content-Type: application/json" \
            --data \
            '{
               "token": "'"$gs_token_2"'",
               "host": "localhost:1234",
               "version": 42,
               "protocol_version": 24
             }' \
             -o "$tmp_dir"/keep-alive-2.json
sed 's/\(callback_delay_seconds":\)[0-9]\+/\1"placeholder"/' \
    -i "$tmp_dir"/keep-alive-2.json
expect_json_eq '{"callback_delay_seconds":"placeholder"}' \
               "$tmp_dir"/keep-alive-2.json

# This one sends incomplete information.
expect_post_error 422 admin/game-servers/keep-alive \
                  -H "Content-Type: application/json" \
                  --data \
                  '{
                     "token": "'"$gs_token"'",
                     "host": "localhost:1234",
                     "version": 42
                   }'
# This one has an unknown token.
expect_post_error 500 admin/game-servers/keep-alive \
            -H "Content-Type: application/json" \
            --data \
            '{
               "token": "some_garbage",
               "host": "localhost:1234",
               "version": 42,
               "protocol_version": 24
             }'
# This one has an invalid host.
expect_post_error 400 admin/game-servers/keep-alive \
            -H "Content-Type: application/json" \
            --data \
            '{
               "token": "'"$gs_token"'",
               "host": "localhost:123456",
               "version": 42,
               "protocol_version": 24
             }'

# The game server should now be online.
expect_get admin/game-servers/list \
           -H "Authorization: $admin_token" \
           -o "$tmp_dir"/list-2.json
sed 's/\(registration_date":"\|last_seen":"\)[^"]\+/\1placeholder/g' \
    -i "$tmp_dir"/list-2.json
expect_json_eq \
    '{
        "valid-server":
        {
          "online": false,
          "description": "Some server.",
          "token": "'"$gs_token"'",
          "last_seen": "placeholder",
          "registration_date": "placeholder"
        },
        "other-server":
        {
          "online": true,
          "description": "Other server.",
          "token": "'"$gs_token_2"'",
          "last_seen": "placeholder",
          "registration_date": "placeholder",
          "info":
          {
            "host": "localhost:1234",
            "version": 42,
            "protocol_version": 24
          }
        }
     }' \
         "$tmp_dir"/list-2.json

# Change the delay between the removal of the game servers for which
# we have no news.
expect_post admin/game-servers/set-time-to-live \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"delay_in_minutes": 0}'
# Can't change the delay if we are no admin.
expect_post_error 401 admin/game-servers/set-time-to-live \
            -H "Authorization: not_an_admin" \
            -H "Content-Type: application/json" \
            --data '{"delay_in_minutes": 10}'
expect_post_error 401 admin/game-servers/set-time-to-live \
            -H "Content-Type: application/json" \
            --data '{"delay_in_minutes": 10}'
expect_post_error 422 admin/game-servers/set-time-to-live \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '10'

# Keep alive, to force an update of the date for removal of this server.
expect_post admin/game-servers/keep-alive \
            -H "Content-Type: application/json" \
            --data \
            '{
               "token": "'"$gs_token_2"'",
               "host": "localhost:1234",
               "version": 42,
               "protocol_version": 24
             }' \
             -o "$tmp_dir"/keep-alive-3.json
sed 's/\(callback_delay_seconds":\)[0-9]\+/\1"placeholder"/' \
    -i "$tmp_dir"/keep-alive-3.json
expect_json_eq '{"callback_delay_seconds":"placeholder"}' \
               "$tmp_dir"/keep-alive-3.json

# Let time pass to trigger a clean-up in the next request.
sleep 2

# All game servers should be offline.
expect_get admin/game-servers/list \
           -H "Authorization: $admin_token" \
           -o "$tmp_dir"/list-3.json
sed 's/\(registration_date":"\|last_seen":"\)[^"]\+/\1placeholder/g' \
    -i "$tmp_dir"/list-3.json
expect_json_eq \
    '{
       "valid-server":
       {
         "online": false,
         "description": "Some server.",
         "token": "'"$gs_token"'",
         "last_seen": "placeholder",
         "registration_date": "placeholder"
       },
       "other-server":
       {
         "online": false,
         "description": "Other server.",
         "token": "'"$gs_token_2"'",
         "last_seen": "placeholder",
         "registration_date": "placeholder"
       }
     }' \
         "$tmp_dir"/list-3.json


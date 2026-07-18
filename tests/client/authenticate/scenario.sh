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
session_token_1="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"
user_id_1="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"

expect_db_row_exists "select * from sessions where token = '$session_token_1'"

# Second client will have very short-living session, to test that
# expired sessions are removed.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "sessions.validity.minutes",
                       "value": "0"
                    }]'

# Authenticate another client for the first time ever.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-2.json
session_token_2="$(jq -r .session_token "$tmp_dir"/authenticate-2.json)"
user_id_2="$(jq -r .user_id "$tmp_dir"/authenticate-2.json)"

expect_db_row_exists "select * from sessions where token = '$session_token_1'"
expect_db_row_exists "select * from sessions where token = '$session_token_2'"

expect_ne "$session_token_1" "$session_token_2"
expect_ne "$user_id_1" "$user_id_2"

# Authenticate the first client again, we should get the same response.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "abc"}' \
            -o "$tmp_dir"/authenticate-3.json
session_token_1b="$(jq -r .session_token "$tmp_dir"/authenticate-3.json)"
user_id_1b="$(jq -r .user_id "$tmp_dir"/authenticate-3.json)"

expect_eq "$session_token_1b" "$session_token_1"
expect_eq "$user_id_1b" "$user_id_1"

# Authenticate the second client again. Its sessions should be expired
# and we should get a new token.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "def"}' \
            -o "$tmp_dir"/authenticate-4.json
session_token_2b="$(jq -r .session_token "$tmp_dir"/authenticate-4.json)"
user_id_2b="$(jq -r .user_id "$tmp_dir"/authenticate-4.json)"

expect_db_row_absent "select * from sessions where token = '$session_token_2'"
expect_db_row_exists "select * from sessions where token = '$session_token_2b'"

expect_ne "$session_token_1b" "$session_token_2b"
expect_ne "$session_token_2" "$session_token_2b"
expect_eq "$user_id_2" "$user_id_2b"

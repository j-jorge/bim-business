#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../../test-functions.sh
start_server

#-------------------------------------------------------------------------------
# Set up.

# Create the administrator.
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
admin_token="$(jq -r . "$tmp_dir"/lead.json)"

# Add more items.
expect_post admin/shop/update \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '[{"id": "id-2", "coins": 22}, {"id": "id-3", "coins": 33}]'

# Authenticate a client.
expect_post client/authenticate \
            --header "Content-Type: application/json" \
            --data '{"device_id": "device-1"}' \
            -o "$tmp_dir"/authenticate-1.json
user_id="$(jq -r .user_id "$tmp_dir"/authenticate-1.json)"
client_token="$(jq -r .session_token "$tmp_dir"/authenticate-1.json)"

#-------------------------------------------------------------------------------
# Actual test.

# Validate a purchase.
expect_post client/billing/validate-purchase \
            --header "Authorization: $client_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "sku": "id-3",
                      "token": "xyz"
                    }' \
                        -o "$tmp_dir"/validate-purchase-1.json
expect_json_eq '{"coins": 33}' "$tmp_dir"/validate-purchase-1.json

# Check that the player has been credited.
expect_post client/wallet \
            --header "Authorization: $client_token" \
            -o "$tmp_dir"/coins-1.json

expect_json_eq '{"coins": 33}' "$tmp_dir"/coins-1.json

expect_db_row_exists \
    'select * from currency_transaction
     where user_id = '"$user_id"'
     and initial_balance = 0
     and amount = 33'

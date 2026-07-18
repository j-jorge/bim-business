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

# Set the game auto removal delay at zero such that the next created
# game is immediately invalid. Same for the reward lifespan, remove it
# as soon as possible.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "games.auto_removal.minutes",
                       "value": "0"
                    },
                    {
                       "key": "games.reward_lifespan.minutes",
                       "value": "0"
                    }]'

# Set the game clean up interval at zero to clean up every time a game
# is created.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[{
                       "key": "games.clean_up_interval.minutes",
                       "value": "0"
                    }]'

# Register a game server.
expect_post admin/game-servers/register \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"name": "gs", "description": "..."}' \
            -o "$tmp_dir"/"gs-1.json"
gs_id="$(jq -r .id "$tmp_dir"/gs-1.json)"
gs_token="$(jq -r .token "$tmp_dir"/gs-1.json)"

# Authenticate some clients.
user_id=()
for i in {0..3}
do
    expect_post client/authenticate \
                --header "Content-Type: application/json" \
                --data '{"device_id": "device-'"$i"'"}' \
                -o "$tmp_dir"/authenticate-"$i".json
    user_id[i]="$(jq -r .user_id "$tmp_dir"/authenticate-"$i".json)"
done

#-------------------------------------------------------------------------------
# Actual tests.

# Start a game, it should be in the table of active games.
expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${user_id[2]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-1.json
game_id_1="$(jq -r .game_id "$tmp_dir"/game-1.json)"
expect_db_row_exists 'select * from game
                      where game_id = '"$game_id_1"'
                      and game_server_id = '"$gs_id"
expect_db_row_exists 'select * from active_game where game_id = '"$game_id_1"

for user in "${user_id[0]}" "${user_id[2]}"
do
    expect_db_row_exists 'select * from active_game_player
                          where game_id = '"$game_id_1"'
                          and user_id = '"$user"
done

# Create another game, it should remove the first one and not consider
# it as done.
expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[1]}"',
                        '"${user_id[3]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-2.json

# The new game is in the database.
game_id_2="$(jq -r .game_id "$tmp_dir"/game-2.json)"
expect_db_row_exists 'select * from game
                      where game_id = '"$game_id_2"'
                      and game_server_id = '"$gs_id"
expect_db_row_exists 'select * from active_game where game_id = '"$game_id_2"

for user in "${user_id[1]}" "${user_id[3]}"
do
    expect_db_row_exists 'select * from active_game_player
                          where game_id = '"$game_id_2"'
                          and user_id = '"$user"
done

# The previous game is not active.
expect_db_row_exists 'select * from game
                      where game_id = '"$game_id_1"'
                      and game_server_id = '"$gs_id"
expect_db_row_absent 'select * from active_game where game_id = '"$game_id_1"
expect_db_row_absent 'select * from done_game where game_id = '"$game_id_1"

for user in "${user_id[0]}" "${user_id[2]}"
do
    expect_db_row_absent 'select * from active_game_player
                          where game_id = '"$game_id_1"'
                          and user_id = '"$user"
    expect_db_row_absent 'select * from done_game_player
                          where game_id = '"$game_id_1"'
                          and user_id = '"$user"
done

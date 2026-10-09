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

# Disable short games rewards and set the rewards.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[
                      {
                        "key": "games.max_duration_for_short_game.seconds",
                        "value": "0"
                      },
                      {
                        "key": "games.coins_per_victory",
                        "value": "2"
                      },
                      {
                        "key": "games.coins_per_defeat",
                        "value": "3"
                      },
                      {
                        "key": "games.coins_per_draw",
                        "value": "7"
                      }
                    ]'


# Register a game server.
expect_post admin/game-servers/register \
            -H "Authorization: $admin_token" \
            -H "Content-Type: application/json" \
            --data '{"name": "gs", "description": "..."}' \
            -o "$tmp_dir"/"gs-1.json"
gs_token="$(jq -r .token "$tmp_dir"/gs-1.json)"

# Authenticate some clients.
user_id=()
client_token=()
for i in {0..3}
do
    expect_post client/authenticate \
                --header "Content-Type: application/json" \
                --data '{"device_id": "device-'"$i"'"}' \
                -o "$tmp_dir"/authenticate-"$i".json
    user_id[i]="$(jq -r .user_id "$tmp_dir"/authenticate-"$i".json)"
    client_token[i]="$(jq -r .session_token "$tmp_dir"/authenticate-"$i".json)"
done

#-------------------------------------------------------------------------------
# Start many games, with the same player in multiple games.

expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${user_id[1]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-1.json
game_id_1="$(jq -r .game_id "$tmp_dir"/game-1.json)"

expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[1]}"',
                        '"${user_id[2]}"',
                        '"${user_id[3]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-2.json
game_id_2="$(jq -r .game_id "$tmp_dir"/game-2.json)"

expect_db_row_exists 'select * from active_game where game_id = '"$game_id_1"
expect_db_row_exists 'select * from active_game where game_id = '"$game_id_2"
expect_db_row_absent 'select * from done_game where game_id = '"$game_id_1"
expect_db_row_absent 'select * from done_game where game_id = '"$game_id_2"

expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"
expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"
expect_db_row_absent 'select * from done_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[2]}"
expect_db_row_absent 'select * from done_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[3]}"

expect_db_row_absent 'select * from done_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[0]}"
expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[1]}"
expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[2]}"
expect_db_row_absent 'select * from done_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[3]}"

expect_post gs/game-over \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"',
                      "duration_in_seconds": 5,
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${user_id[1]}"'
                      ],
                      "outcome": ["defeated", "victory"]
                    }'

expect_db_row_exists 'select * from done_game
                      where game_id = '"$game_id_1"'
                      and short_game = false'
expect_db_row_absent 'select * from done_game
                      where game_id = '"$game_id_2"'
                      and short_game = false'

expect_db_row_absent 'select * from active_game where game_id = '"$game_id_1"
expect_db_row_exists 'select * from active_game where game_id = '"$game_id_2"

expect_db_row_absent 'select * from active_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"
expect_db_row_absent 'select * from active_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"

expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[1]}"
expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[2]}"
expect_db_row_exists 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[3]}"

expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"

expect_post gs/game-over \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_2"',
                      "duration_in_seconds": 50,
                      "players":
                      [
                        '"${user_id[1]}"',
                        '"${user_id[2]}"',
                        '"${user_id[3]}"'
                      ],
                      "outcome": ["defeated", "victory", "defeated"]
                    }'

# TODO: rm 2
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"

expect_db_row_absent 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[1]}"
expect_db_row_absent 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[2]}"
expect_db_row_absent 'select * from active_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[3]}"

expect_db_row_unique 'from done_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"'
                      and outcome = '"'Defeated'"
expect_db_row_unique 'from done_game_player
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"'
                      and outcome = '"'Victory'"

expect_db_row_unique 'from done_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[1]}"'
                      and outcome = '"'Defeated'"
expect_db_row_unique 'from done_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[2]}"'
                      and outcome = '"'Victory'"
expect_db_row_unique 'from done_game_player
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[3]}"'
                      and outcome = '"'Defeated'"

expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[1]}"
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[2]}"
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_2"'
                      and user_id = '"${user_id[3]}"

expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[0]}"
expect_db_row_exists 'select * from game_reward
                      where game_id = '"$game_id_1"'
                      and user_id = '"${user_id[1]}"

echo "users:"
printf "%s\n" "${user_id[@]}"
echo "tokens:"
printf "%s\n" "${client_token[@]}"

expect_post client/game/consume-reward \
            --header "Authorization: ${client_token[1]}" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"'
                    }' \
                        -o "$tmp_dir"/reward-1.json
expect_json_eq '{"coins": 2}' "$tmp_dir"/reward-1.json

expect_post client/game/consume-reward \
            --header "Authorization: ${client_token[1]}" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_2"'
                    }' \
                        -o "$tmp_dir"/reward-2.json
expect_json_eq '{"coins": 3}' "$tmp_dir"/reward-2.json

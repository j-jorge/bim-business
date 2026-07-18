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

# Force short games and set the rewards.
expect_post admin/app-config/update \
            --header "Authorization: $admin_token" \
            --header "Content-Type: application/json" \
            --data '[
                      {
                        "key": "games.max_duration_for_short_game.seconds",
                        "value": "10000"
                      },
                      {
                        "key": "games.coins_per_short_game_victory",
                        "value": "2"
                      },
                      {
                        "key": "games.coins_per_short_game_defeat",
                        "value": "3"
                      },
                      {
                        "key": "games.coins_per_short_game_draw",
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
for i in {0..3}
do
    expect_post client/authenticate \
                --header "Content-Type: application/json" \
                --data '{"device_id": "device-'"$i"'"}' \
                -o "$tmp_dir"/authenticate-"$i".json
    user_id[i]="$(jq -r .user_id "$tmp_dir"/authenticate-"$i".json)"
done

#-------------------------------------------------------------------------------
# Check the rewards.

expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${user_id[3]}"',
                        '"${user_id[2]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-1.json
game_id_1="$(jq -r .game_id "$tmp_dir"/game-1.json)"

expect_post gs/game-over \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"',
                      "has_a_winner": true,
                      "players":
                      [
                        '"${user_id[2]}"',
                        '"${user_id[3]}"',
                        '"${user_id[0]}"'
                      ],
                      "player_ranks": [2, 0, 0]
                    }'
expect_db_row_exists 'select * from done_game
                      where game_id = '"$game_id_1"'
                      and short_game = true'

expect_db 'select * from game_reward
           where game_id = '"$game_id_1"'
           and user_id = '"${user_id[2]}" \
               "$tmp_dir"/reward-g1-user-2.txt
expect_true grep --quiet '^coins *| *3$' "$tmp_dir"/reward-g1-user-2.txt

expect_db 'select * from game_reward
           where game_id = '"$game_id_1"'
           and user_id = '"${user_id[3]}" \
               "$tmp_dir"/reward-g1-user-3.txt
expect_true grep --quiet '^coins *| *7$' "$tmp_dir"/reward-g1-user-3.txt

expect_db 'select * from game_reward
           where game_id = '"$game_id_1"'
           and user_id = '"${user_id[0]}" \
               "$tmp_dir"/reward-g1-user-0.txt
expect_true grep --quiet '^coins *| *7$' "$tmp_dir"/reward-g1-user-0.txt

#-------------------------------------------------------------------------------
# Check the rewards when Everybody loses.

expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        '"${user_id[2]}"',
                        '"${user_id[1]}"',
                        '"${user_id[0]}"',
                        '"${user_id[3]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-2.json
game_id_2="$(jq -r .game_id "$tmp_dir"/game-2.json)"

expect_post gs/game-over \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_2"',
                      "has_a_winner": false,
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${user_id[1]}"',
                        '"${user_id[2]}"',
                        '"${user_id[3]}"'
                      ],
                      "player_ranks": [0, 0, 0, 0]
                    }'
expect_db_row_exists 'select * from done_game
                      where game_id = '"$game_id_2"'
                      and short_game = true'

for i in {0..3}
do
    expect_db 'select * from game_reward
           where game_id = '"$game_id_2"'
           and user_id = '"${user_id[i]}" \
              "$tmp_dir"/reward-g2-user-"$i".txt
    expect_true grep --quiet '^coins *| *7$' "$tmp_dir"/reward-g2-user-"$i".txt
done

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

# Set the rewards.
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
                        "value": "7"
                      },
                      {
                        "key": "games.coins_per_defeat",
                        "value": "2"
                      },
                      {
                        "key": "games.coins_per_draw",
                        "value": "3"
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
for i in {0..2}
do
    expect_post client/authenticate \
                --header "Content-Type: application/json" \
                --data '{"device_id": "device-'"$i"'"}' \
                -o "$tmp_dir"/authenticate-"$i".json
    user_id[i]="$(jq -r .user_id "$tmp_dir"/authenticate-"$i".json)"
    client_token[i]="$(jq -r .session_token "$tmp_dir"/authenticate-"$i".json)"
done

#-------------------------------------------------------------------------------
# Pretend a game has been played.

expect_post gs/game-started \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "players":
                      [
                        0,
                        '"${user_id[0]}"',
                        '"${user_id[1]}"',
                        '"${user_id[2]}"'
                      ]
                    }' \
                        -o "$tmp_dir"/game-1.json
game_id_1="$(jq -r .game_id "$tmp_dir"/game-1.json)"
readarray -t players_1 < <(jq -r '.players[]' "$tmp_dir"/game-1.json)

expect_post gs/game-over \
            --header "Authorization: $gs_token" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"',
                      "has_a_winner": true,
                      "players":
                      [
                        '"${user_id[0]}"',
                        '"${players_1[0]}"',
                        '"${user_id[2]}"'
                      ],
                      "player_ranks": [0, 1, 2]
                    }'

expect_post client/game/consume-reward \
            --header "Authorization: ${client_token[0]}" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"'
                    }' \
                        -o "$tmp_dir"/reward-0.json
expect_json_eq '{"coins": 7}' "$tmp_dir"/reward-0.json

# Once it's consumed it cannot be consumed again.
expect_post_error 400 client/game/consume-reward \
                  --header "Authorization: ${client_token[0]}" \
                  --header "Content-Type: application/json" \
                  --data '{
                      "game_id": '"$game_id_1"'
                    }' \
                        -o "$tmp_dir"/reward-0.json

expect_post_error 400 client/game/consume-reward \
                  --header "Authorization: ${client_token[1]}" \
                  --header "Content-Type: application/json" \
                  --data '{
                      "game_id": '"$game_id_1"'
                    }' \
                        -o "$tmp_dir"/reward-1.json

expect_post client/game/consume-reward \
            --header "Authorization: ${client_token[2]}" \
            --header "Content-Type: application/json" \
            --data '{
                      "game_id": '"$game_id_1"'
                    }' \
                        -o "$tmp_dir"/reward-2.json
expect_json_eq '{"coins": 2}' "$tmp_dir"/reward-2.json

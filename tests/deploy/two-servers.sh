#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh

prod_dir="$tmp_dir"/prod-test
stage_dir="$tmp_dir"/stage-test

expect_false test -d "$prod_dir"
expect_false test -d "$stage_dir"

wait_server_up()
{
    # Clear the log, otherwise wait_server_ready will find immediately
    # find the trace from the previous run.
    if [[ -f "$tmp_dir"/"$1"/bim/host/logs/bim-business.stdout.txt ]]
    then
        true > "$tmp_dir"/"$1"/bim/host/logs/bim-business.stdout.txt
    fi

    wait_server_ready \
        "$tmp_dir"/"$1"/bim/host/logs/bim-business.stdout.txt \
        "$tmp_dir"/"$1"/bim/host/logs/bim-business.stderr.txt
}

# This function will stop the servers when the script exits.
stop_servers_on_exit()
{
    count_failure_on_script_error

    cd "$prod_dir"
    # Use the same project name than the one implicitly used by deploy.sh,
    # otherwise the up & down commands won't match what has been
    # instantiated by deploy.sh.
    docker compose --project-name bim-business-prod-test down

    cd "$stage_dir"
    docker compose --project-name bim-business-stage-test down
}

push_on_exit stop_servers_on_exit
prod_port="$app_port"
stage_port="$((app_port * 2))"

cat > "$prod_dir".conf <<EOF
bim_db_password=test-password
bim_db_name=test-db
bim_db_user=test-user
bim_port=$prod_port
EOF

cat > "$stage_dir".conf <<EOF
bim_db_password=test-password
bim_db_name=test-db
bim_db_user=test-user
bim_port=$stage_port
EOF

info "Deploy and start the prod server."
expect_true "$repo_root"/deploy/deploy.sh \
                --build-type "$build_type" \
                --config "$prod_dir".conf \
                --destination-root "$tmp_dir" \
                --tag prod-test
wait_server_up prod-test

info "Deploy and start the stage server."
expect_true "$repo_root"/deploy/deploy.sh \
                --build-type "$build_type" \
                --config "$stage_dir".conf \
                --destination-root "$tmp_dir" \
                --dev \
                --tag stage-test
wait_server_up stage-test

info "Interact with prod."
expect_true curl \
            --silent \
            --show-error \
            --fail \
            --request POST \
            --header "Authorization: _" \
            "http://localhost:$prod_port/admin/leads/create" \
            --output /dev/null
expect_true curl \
            --silent \
            --show-error \
            --fail \
            --request POST \
            --header "Content-Type: application/json" \
            "http://localhost:$prod_port/client/config" \
            --data '{"game_server_protocol_version": 22}'

info "Interact with stage."
expect_true curl \
            --silent \
            --show-error \
            --fail \
            --request POST \
            --header "Authorization: _" \
            "http://localhost:$stage_port/admin/leads/create" \
            --output /dev/null
expect_true curl \
            --silent \
            --show-error \
            --fail \
            --request POST \
            --header "Content-Type: application/json" \
            "http://localhost:$stage_port/client/config" \
            --data '{"game_server_protocol_version": 42}'

#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/../test-functions.sh

deploy_dir="$tmp_dir"/testing
expect_false test -d "$deploy_dir"

docker_compose()
{
    cd "$deploy_dir"
    expect_true docker compose "$@"
    cd -
}

wait_server_up()
{
    # Clear the log, otherwise wait_server_ready will find immediately
    # find the trace from the previous run.
    if [[ -f "$deploy_dir"/bim/host/logs/bim-business.stdout.txt ]]
    then
        true > "$deploy_dir"/bim/host/logs/bim-business.stdout.txt
    fi

    wait_server_ready \
        "$deploy_dir"/bim/host/logs/bim-business.stdout.txt \
        "$deploy_dir"/bim/host/logs/bim-business.stderr.txt
}

# Use the same project name than the one implicitly used by deploy.sh,
# otherwise the up & down commands won't match what has been
# instantiated by deploy.sh.
docker_project_name=bim-business-testing

start_server()
{
    docker_compose --project-name "$docker_project_name" up --detach
    wait_server_up
}

stop_server()
{
    docker_compose --project-name "$docker_project_name" down
}

# This function will stop the server when the script exits.
stop_server_on_exit()
{
    count_failure_on_script_error
    stop_server
}

push_on_exit stop_server_on_exit

cat > "$tmp_dir"/testing.conf <<EOF
bim_db_password=test-password
bim_db_name=test-db
bim_db_user=test-user
bim_port=$app_port
EOF

deploy_command=("$repo_root"/deploy/deploy.sh
                --build-type "$build_type"
                --config "$tmp_dir"/testing.conf
                --destination-root "$tmp_dir"
                --tag testing
               )

info "Deploy and start the server."
expect_true "${deploy_command[@]}"
expect_true test -f "$deploy_dir"/bim/bin/bim-business
wait_server_up

info "Populate the server with some info."
expect_post admin/leads/create --header "Authorization: _" \
            -o "$tmp_dir"/lead.json
lead_token="$(jq -r . "$tmp_dir"/lead.json)"

expect_post admin/game-features/update \
            --header "Authorization: $lead_token" \
            --header "Content-Type: application/json" \
            --data '{"item-1": 11}'

info "Deploy again, it should fail because the lock is active."
expect_false "${deploy_command[@]}"

expect_true rm --force "$deploy_dir"/lock

info "Now that the lock is removed, deploy again. It should stop the server"
info "and create an archive with its state before the update."
expect_true "${deploy_command[@]}"

# There should be one archive.
readarray -t archives < <(find "$tmp_dir" -maxdepth 1 -name "*.tgz")

expect_eq 1 "${#archives[@]}"

# All files from the server should be owned by the user who launched
# it. By default the postgres Docker image create files as root, this
# test ensures that we worked around it.
expect_true find "$deploy_dir" '!' -user "$(whoami)" -exec false {} +

info "Now we stop the server and delete its data before restarting it."
stop_server
expect_true rm --force --recursive "$deploy_dir"/db/pgdata

start_server

info "All the data has been removed, thus our lead token should be refused."
expect_get_error 401 admin/leads/list --header "Authorization: $lead_token"

info "Restore the archive"
stop_server

expect_true rm --force --recursive "$deploy_dir"/db/pgdata
cd "$tmp_dir"/
expect_true tar xf "${archives[0]}" testing/db/pgdata
cd - >/dev/null

start_server

info "The data is back, thus our lead token should be accepted."
expect_get admin/leads/list --header "Authorization: $lead_token" \
     -o "$tmp_dir"/list-1.json
expect_json_eq '["'"$lead_token"'"]' "$tmp_dir"/list-1.json

info "Deploy again, it should stop the server and create a new archive with"
info "its state before the update."
expect_true rm --force "$deploy_dir"/lock
expect_true "${deploy_command[@]}"

# There should be two archives.
readarray archives < <(find "$tmp_dir" -maxdepth 1 -name "*.tgz")

expect_eq 2 "${#archives[@]}"

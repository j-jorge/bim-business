#!/bin/bash

set -euo pipefail

usage()
{
    cat <<EOF
Start the business server of Bim! and print its PID on stdout.

$0 server_binary container_name user password db_name db_port app_port tmp_dir

server_binary
  Path to the bim-business binary to launch.
container_name
  Name to assign to the Docker container.
user
  Name to assign to the database user.
password
  The password for the database user.
db_name
  Name to assign to the database.
db_port
  The port on which the database server listens.
app_port
  The port on which the app listens.
tmp_dir
  Where to put the produced files.
EOF
}

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    usage
    exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"
repo_root="$(cd "$(dirname "$script_dir")"; pwd)"

server_binary="$1"
db_container_name="$2"
db_user="$3"
db_password="$4"
db_name="$5"
db_port="$6"
app_port="$7"
tmp_dir="$8"

server_pid=""

# Start the database and wait for it to be up and ready.
docker run --rm --name "$db_container_name" \
       --env POSTGRES_USER="$db_user" \
       --env POSTGRES_PASSWORD="$db_password" \
       --env POSTGRES_DB="$db_name" \
       --publish 5432:"$db_port" \
       postgres:18 \
       > "$tmp_dir"/postgres.out.txt \
       2> "$tmp_dir"/postgres.err.txt \
    &

if "$script_dir"/wait-poll-file.sh 60 \
                "$tmp_dir"/postgres.err.txt \
                "ready to accept connections" \
                "$tmp_dir"/postgres.out.txt
then
    cat > "$tmp_dir"/secrets.json <<EOF
{
  "db_password": "$db_password"
}
EOF

    # Now that he database is up the server can start.
    "$server_binary" \
        --assets "$repo_root"/assets \
        --port "$app_port" \
        --db-port "$db_port" \
        --db-name "$db_name" \
        --db-user "$db_user" \
        --secrets "$tmp_dir"/secrets.json \
        > "$tmp_dir"/server.out.txt \
        2> "$tmp_dir"/server.err.txt \
        &

    server_pid=$!

    "$script_dir"/wait-poll-file.sh \
                 60 \
                 "$tmp_dir"/server.out.txt \
                 'Starting the web services' \
                 "$tmp_dir"/server.err.txt

    if [[ -s "$tmp_dir"/server.err.txt ]]
    then
        cat "$tmp_dir"/server.err.txt
        exit 1
    fi
fi

echo "PID=$server_pid"

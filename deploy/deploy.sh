#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

function usage()
{
    cat <<EOF
Usage: "${BASH_SOURCE[0]}" OPTIONS

Where OPTIONS is
  --build-type [ debug | release ]
     Mandatory. The build to upload.
  --config FILE
     The config file from which we get the configuration of this
     script for the app.
  -h, --help
     Display this message and exit.
EOF
}

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    usage
    exit 0
fi

while [[ $# -ne 0 ]]
do
    arg="$1"
    shift

    case "$arg" in
        --build-type)
            if [[ "$#" -eq 0 ]]
            then
                echo "Missing value for --build-type." >&2
                exit 1
            fi
            build_type="$1"
            shift
            ;;
        --config)
            if [[ "$#" -eq 0 ]]
            then
                echo "Missing value for --config." >&2
                exit 1
            fi
            config_file="$1"
            shift
            ;;
    esac
done

if [[ -z "${build_type:-}" ]]
then
    echo "--build_type is required." >&2
    exit 1
fi

if [[ -z "${config_file:-}" ]]
then
    echo "--config is required." >&2
    exit 1
fi

tmp_dir="$(mktemp --directory)"

function clean_up()
{
    rm --force --recursive "$tmp_dir"
}

trap clean_up EXIT

function destination_exec()
{
    if [[ -n "${bim_host:-}" ]]
    then
        # We want $1 to expand on the client side.
        # shellcheck disable=SC2029
        ssh "$bim_host" "$1"
    else
        bash -c "$1"
    fi
}

# shellcheck disable=SC1090
. "$config_file"

if [[ -z "${bim_tag:-}" ]]
then
    echo "bim_tag must be set."
    exit 1
fi

if [[ -z "${bim_destination_root:-}" ]]
then
    echo "bim_destination_root must be set."
    exit 1
fi

bim_prod_or_dev="${bim_prod_or_dev:-prod}"

# Aggregate the files to deploy
archive_path="$tmp_dir"/bim-business-"$bim_tag"

mkdir --parents "$archive_path"/bim/{bin,etc,host}
cp "$script_dir"/docker-compose.yml \
   "$script_dir"/dockerfile.* \
   "$archive_path"/
cp --recursive \
   "$script_dir"/../assets \
   "$archive_path"/bim/
cp "$script_dir"/../target/"$build_type"/bim-business \
   "$script_dir"/bim-business-launcher.sh \
   "$archive_path"/bim/bin/

if [[ -z "${bim_host:-}" ]]
then
    mkdir --parents "$bim_destination_root"
fi

destination_path="$bim_destination_root"/"$bim_tag"

# All those variables are expected to be set by the config file.
#
# shellcheck disable=SC2154
cat > "$archive_path"/.env <<EOF
BIM_DB_PASSWORD="$bim_db_password"
BIM_DB_NAME="$bim_db_name"
BIM_DB_USER="$bim_db_user"
BIM_TAG="$bim_tag"
BIM_PORT=$bim_port
BIM_CLIENT_APP_ID="$bim_client_app_id"
EOF

json_db_password="$(echo "${bim_db_password}" \
                         | jq --raw-input --raw-output --ascii-output .)"
cat > "$archive_path"/bim/etc/secrets.json <<EOF
{"db_password": $json_db_password}
EOF

# bim_google_cloud_credentials is set by the config file.
#
# shellcheck disable=SC2154
cp "$bim_google_cloud_credentials" "$archive_path"/bim/etc/googleapi.json

# A script to start the new server
cat > "$archive_path"/"bim-business-launch.sh" <<EOF
#!/bin/bash

set -euo pipefail
cd "$destination_path"/

(
  echo "UID=\$(id --user)"
  echo "GID=\$(id --group)"
) >> .env

docker compose --project-name bim-business-"$bim_tag" up --detach

if [[ "$bim_prod_or_dev" = prod ]]
then
    touch lock
fi
EOF

chmod u+x "$archive_path"/bim-business-launch.sh

now="$(date --iso-8601=seconds | tr -d ':')"

# A script to prepare the deployment of the server: we stop the old
# server and install the new files.
cat > "$tmp_dir"/"bim-business-pre-deploy.sh" <<EOF
#!/bin/bash

set -euo pipefail

if [[ -e "$destination_path"/lock ]]
then
    echo "'$destination_path/lock' exists. Aborting."
    exit 1
fi

mkdir --parents "$destination_path"/bim/host \
                "$destination_path"/db/{host,pgdata}

cd "$destination_path"/

if [[ -f docker-compose.yml ]]
then
     docker compose --project-name bim-business-"$bim_tag" down

     cd ..
     tar cfz "$bim_tag-$now.tgz" "$bim_tag"
fi
EOF

# Prepare the remote
chmod u+x "$tmp_dir"/bim-business-pre-deploy.sh

if [[ -n "${bim_host:-}" ]]
then
    rsync --progress "$tmp_dir"/bim-business-pre-deploy.sh "$bim_host:/tmp/"
    destination_exec "/tmp/bim-business-pre-deploy.sh && \
                     rm /tmp/bim-business-pre-deploy.sh"
    rsync --progress "$bim_host":"$destination_path/../$bim_tag-$now.tgz" .
else
    "$tmp_dir"/bim-business-pre-deploy.sh
fi

# Copy the aggregated files to the destination dir.
if [[ -n "${bim_host:-}" ]]
then
    rsync --progress --recursive "$archive_path"/ "$bim_host:$destination_path/"
else
    mkdir --parents "$destination_path"
    rsync --progress --recursive "$archive_path"/ "$destination_path/"
fi

# And finally start the server.
destination_exec "cd '$destination_path' \
                 && ./bim-business-launch.sh \
                 && rm --force ./bim-business-launch.sh"

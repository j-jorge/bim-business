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
     The config file from which we get the configuration of the app.
  --destination-root PATH
     The root folder where the deployment is done. The archive will be
     created in this directory but the server will be deployed in
     --tag interpreted as a subdirectory of PATH.
  --dev
     This is a developer deployment, it won't be guarded against
     accidental replacement.
  -h, --help
     Display this message and exit.
  --host
     The host that will receive the files. If no host is given, the
     deployment is done on localhost.
  --tag NAME
     A tag to give to this service. This will be used as a directory
     in --destination-root, into which the server will be deployed.
EOF
}

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    usage
    exit 0
fi

prod_or_dev=prod

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
        --destination-root)
            if [[ "$#" -eq 0 ]]
            then
                echo "Missing value for --destination-root." >&2
                exit 1
            fi
            destination_root="$1"
            shift
            ;;
        --host)
            if [[ "$#" -eq 0 ]]
            then
                echo "Missing value for --host." >&2
                exit 1
            fi
            host="$1"
            shift
            ;;
        --dev)
            prod_or_dev=dev
            ;;
        --tag)
            if [[ "$#" -eq 0 ]]
            then
                echo "Missing value for --tag." >&2
                exit 1
            fi
            tag="$1"
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

if [[ -z "${destination_root:-}" ]]
then
    echo "--destination-root is required." >&2
    exit 1
fi

if [[ -z "${tag:-}" ]]
then
    echo "--tag is required." >&2
    exit 1
fi

if [[ -z "${host:-}" ]]
then
    mkdir --parents "$destination_root"
fi

destination_path="$destination_root"/"$tag"

tmp_dir="$(mktemp --directory)"

function clean_up()
{
    rm --force --recursive "$tmp_dir"
}

trap clean_up EXIT

function destination_exec()
{
    if [[ -n "${host:-}" ]]
    then
        # We want $1 to expand on the client side.
        # shellcheck disable=SC2029
        ssh "$host" "$1"
    else
        bash -c "$1"
    fi
}

# Aggregate the files to deploy
archive_path="$tmp_dir"/bim-business-"$prod_or_dev"

mkdir --parents "$archive_path"/bim/{bin,etc,host}
cp "$script_dir"/docker-compose.yml \
   "$script_dir"/dockerfile.db \
   "$archive_path"/
cp --recursive \
   "$script_dir"/../assets \
   "$archive_path"/bim/
cp "$script_dir"/../target/"$build_type"/bim-business \
   "$script_dir"/bim-business-launcher.sh \
   "$archive_path"/bim/bin/

# shellcheck disable=SC1090
. "$config_file"

# All those variables are expected to be set by the config file.
#
# shellcheck disable=SC2154
cat > "$archive_path"/.env <<EOF
BIM_DB_PASSWORD="$bim_db_password"
BIM_DB_NAME="$bim_db_name"
BIM_DB_USER="$bim_db_user"
BIM_TAG="$tag"
BIM_PORT=$bim_port
EOF

json_db_password="$(echo "${bim_db_password}" \
                         | jq --raw-input --raw-output --ascii-output .)"
cat > "$archive_path"/bim/etc/secrets.json <<EOF
{"db_password": $json_db_password}
EOF

# A script to start the new server
cat > "$archive_path"/"bim-business-launch.sh" <<EOF
#!/bin/bash

set -euo pipefail
cd "$destination_path"/

(
  echo "UID=\$(id --user)"
  echo "GID=\$(id --group)"
) >> .env

docker compose --project-name bim-business-"$tag" up --detach

if [[ "$prod_or_dev" = prod ]]
then
    touch lock
fi
EOF

chmod u+x "$archive_path"/bim-business-launch.sh

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
     docker compose --project-name bim-business-"$tag" down

     cd ..
     date="\$(date --iso-8601=seconds | tr -d ':')"
     tar cfz "$tag-\$date.tgz" "$tag"
fi
EOF

# Prepare the remote
chmod u+x "$tmp_dir"/bim-business-pre-deploy.sh

if [[ -n "${host:-}" ]]
then
    rsync --progress "$tmp_dir"/bim-business-pre-deploy.sh "$host:/tmp/"
    destination_exec "/tmp/bim-business-pre-deploy.sh && \
                     rm /tmp/bim-business-pre-deploy.sh"
else
    "$tmp_dir"/bim-business-pre-deploy.sh
fi

# Copy the aggregated files to the destination dir.
if [[ -n "${host:-}" ]]
then
    rsync --progress --recursive "$archive_path"/ "$host:$destination_path/"
else
    mkdir --parents "$destination_path"
    rsync --progress --recursive "$archive_path"/ "$destination_path/"
fi

# And finally start the server.
destination_exec "cd '$destination_path' \
                 && ./bim-business-launch.sh \
                 && rm --force ./bim-business-launch.sh"

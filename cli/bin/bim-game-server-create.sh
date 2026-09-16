#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} --name NAME --description DESCRIPTION
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

while [[ $# -ne 0 ]]
do
    arg="$1"
    shift

    case "$arg" in
        --name)
            ensure_argument_has_value "--name" "$@"
            name="$1"
            shift
            ;;
        --description)
            ensure_argument_has_value "--description" "$@"
            description="$1"
            shift
            ;;
        *)
            unknown_argument "$arg"
            ;;
    esac
done

check_argument "--name" name
check_argument "--description" description

bim_curl admin/game-servers/register \
         --header "Content-Type: application/json" \
         --data '{"name": "'"$name"'", "description": "'"$description"'"}'

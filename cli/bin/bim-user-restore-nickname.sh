#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} USER
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

if (( $# != 1 ))
then
    usage
    exit 1
fi

data="$(printf '{"user_id": %s}' "$@")"

bim_curl admin/users/restore-nickname \
         --header "Content-Type: application/json" \
         --data "$data"

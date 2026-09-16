#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} USER NICKNAME
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

if (( $# != 2 ))
then
    usage
    exit 1
fi

data="$(printf '{"user_id": %s, "nickname": "%s"}' "$@")"

bim_curl admin/users/override-nickname \
         --header "Content-Type: application/json" \
         --data "$data"

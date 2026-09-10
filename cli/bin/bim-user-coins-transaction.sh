#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} USER AMOUNT REASON
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

if (( $# != 3 ))
then
    usage
    exit 1
fi

data="$(printf '{
  "user_id": %s,
  "amount": %s,
  "reason": "%s"
}' "$@")"

bim_curl admin/users/coins-transaction \
         --header "Content-Type: application/json" \
         --data "$data"

#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} [ INDEX COST ]…
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

if (( $# % 2 != 0 )) || (( $# == 0 ))
then
    echo "Pass pairs of slot index and cost as arguments." >&2
    exit 1
fi

data='['
separator=""

while [[ $# != 0 ]]
do
    data+="$(printf '%s{"index": %d, "coins": %d}' "$separator" "$1" "$2")"
    separator=','
    shift 2
done

data+=']'

bim_curl admin/game-feature-slots/update \
         --header "Content-Type: application/json" \
         --data "$data"

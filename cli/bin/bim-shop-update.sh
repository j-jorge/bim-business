#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} [ ID COINS ]…
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

if (( $# % 2 != 0 )) || (( $# == 0 ))
then
    echo "Pass pairs of item ID and coins as arguments." >&2
    exit 1
fi

data='['
separator=""

while [[ $# != 0 ]]
do
    data+="$(printf '%s{"id": "%s", "coins": %d}' "$separator" "$1" "$2")"
    separator=','
    shift 2
done

data+=']'

bim_curl admin/shop/update \
         --header "Content-Type: application/json" \
         --data "$data"

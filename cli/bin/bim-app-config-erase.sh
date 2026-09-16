#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} KEY…
EOF
}

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

data='['
separator=""

while [[ $# != 0 ]]
do
    data+="$separator"'"'"$1"'"'
    separator=','
    shift
done

data+=']'

bim_curl admin/app-config/erase \
         --header "Content-Type: application/json" \
         --data "$data"

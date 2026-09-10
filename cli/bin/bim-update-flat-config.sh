#!/bin/bash

usage()
{
    cat <<EOF
Usage: ${BASH_SOURCE[0]} [ KEY VALUE ]…

Value must be quoted if it is going to be stored as a string, i.e. you
must pass '"string_value"'.
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

data='{'
separator=""

while [[ $# != 0 ]]
do
    data+="$separator"'"'"$1"'":'"$2"
    separator=','
    shift 2
done

data+='}'

bim_curl admin/flat-client-config/update \
         --header "Content-Type: application/json" \
         --data "$data"

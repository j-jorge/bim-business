#!/bin/bash

set -euo pipefail

if printf '%s\n' "$@" | grep --quiet "^\(-h\|--help\)"
then
    usage
    exit 0
fi

if [[ -z "${BIM_BUSINESS_SERVER_URL:-}" ]]
then
    echo "BIM_BUSINESS_SERVER_URL is not set. Aborting." >&2
    exit 1
fi

if [[ "${lead_required:-0}" -eq 1 ]] && [[ -z "${BIM_BUSINESS_LEAD_TOKEN:-}" ]]
then
    echo "BIM_BUSINESS_LEAD_TOKEN is not set. Aborting." >&2
    exit 1
fi

# First argument is the command-line option to check, the rest should
# be "@" after the argument name has been consumed, because the
# function has no access to $@ from the caller otherwise, and the
# caller cannot pass the next argument if it is not defined.
ensure_argument_has_value()
{
    local name="$1"

    if [[ ! -v 2 ]]
    then
        echo "Missing value for $name." >&2
        exit 1
    fi
}

check_argument()
{
    local name="$1"
    local -n value="$2"

    if [[ -z "${value:-}" ]]
    then
        echo "$name must be set." >&2
        exit 1
    fi
}

unknown_argument()
{
    echo "Unhandled argument '$1'" >&2
    exit 1
}

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

bim_curl()
{
    echo "Working with server $BIM_BUSINESS_SERVER_URL."

    local path="$1"
    shift

    command=(curl --fail --silent "$BIM_BUSINESS_SERVER_URL"/"$path")

    if [[ "${lead_required:-0}" -eq 1 ]]
    then
        command+=(--header "Authorization: $BIM_BUSINESS_LEAD_TOKEN")
    fi

    "${command[@]}" "$@" | jq --sort-keys .
}

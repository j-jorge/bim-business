#!/bin/bash

set -euo pipefail

echo "$@"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/colors.sh

fail_count=0
failing=()
test_args=()
filter=""

while [[ $# -ne 0 ]]
do
    arg="$1"
    shift

    case "$arg" in
        --filter)
            filter="$1"
            shift
            ;;
        *)
            test_args+=("$arg")
            ;;
    esac
done

function filter_match()
{
    # We want to glob-match the filter, thus it must not be quoted.
    # shellcheck disable=SC2053
    [[ "$1" = $filter ]]
}

while read -r test_script
do
    if [[ -z "$filter" ]] || filter_match "$test_script"
    then
        if ! "$test_script" "${test_args[@]}"
        then
            fail_count=$((fail_count + 1))
            failing+=("$test_script")
        fi
    fi
done < <(find "$script_dir" -mindepth 2 -executable -name "*.sh")

if (( fail_count == 0 ))
then
    echo -e "${green}[======]$reset_color"
    echo -e "${green}[ PASS ]$reset_color All tests passed."
else
    echo -e "${red}[======]$reset_color"
    echo -e "${red}[ FAIL ]$reset_color $fail_count failures:"
    printf '  %s\n' "${failing[@]}"
    exit 1
fi

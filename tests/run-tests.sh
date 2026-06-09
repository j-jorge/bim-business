#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"

# shellcheck source-path=SCRIPTDIR
. "$script_dir"/colors.sh

fail_count=0
failing=()

while read -r test_script
do
    if ! "$test_script" "$@"
    then
        fail_count=$((fail_count + 1))
        failing+=("$test_script")
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

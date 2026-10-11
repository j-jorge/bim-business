#!/bin/bash

set -euo pipefail

usage()
{
    cat <<EOF
Wait for a given file to contain the given regular expression, or up
to a given timeout. If the timeout is reached the function dumps the
content of the file as well as a provided error/second file.

$0 delay file regex second_file

delay
  How many seconds to wait.
file
  The file to observe.
regex
  The regular expression to search in the file.
second_file
  Another file to dump on error.
EOF
}

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    usage
    exit 0
fi

# How many seconds to wait.
seconds="$1"

# The file to observe.
f="$2"

# The regular expression to search in the file.
regex="$3"

# Another file to dump on error.
second_file="$4"

echo "Waiting for '$regex' in '$f'."

while (( seconds >= 1 ))
do
    if [[ -f "$f" ]] && grep --quiet "$regex" "$f"
    then
        exit 0
    fi

    seconds=$((seconds - 1))
    sleep 1
done

echo "Could not find pattern '$regex' in '$f':"
cat "$f"

if [[ -s "${second_file:-}" ]]
then
    echo "Second file:"
    cat "$second_file"
fi

exit 1

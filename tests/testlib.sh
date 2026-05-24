# shellcheck shell=bash
#
# Source this script to get access to the test functions.
#

_testlib_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"
_testlib_script_dir="$(readlink --canonicalize "$_testlib_script_dir")"

# shellcheck source-path=SCRIPTDIR
. "$_testlib_script_dir"/colors.sh

# The test name is basically the script name.
test_name="$(readlink --canonicalize "$0")"
test_name="${test_name/$_testlib_script_dir\//}"

# How many test did fail. Exit on error if this is not zero when the
# test suite ends.
fail_count=0

_print_line_header()
{
    local header_size=6

    printf '['

    if [[ ${#1} -eq 1 ]]
    then
        for _ in {1..6}
        do
            printf '%s' "$1"
        done
    elif [[ ${#1} -le "$header_size" ]]
    then
        local left=$(( (header_size - ${#1}) / 2 ))
        printf '%*s%s%*s' "$left" "" "$1" $((header_size - left - ${#1})) ""
    else
        printf '%s' "$1"
    fi

    printf ']'
}

_message()
{
    echo -e -n "$1"
    _print_line_header "$2"
    shift 2

    echo -e "$reset_color" "$@"
}

pass()
{
    _message "$green" PASS "$@"
}

empty_ok()
{
    _message "$green" "" "$@"

}

fail()
{
    _message "$red" FAIL "$@"
}

info()
{
    _message "$yellow" INFO "$@"
}

_message "$green" =
empty_ok "Starting $test_name"

_print_results()
{
    if (( $? != 0 ))
    then
        fail "$test_name: script failed"
        fail_count=$((fail_count + 1))
        return 1
    elif (( fail_count == 0 ))
    then
        _message "$green" -
        pass "$test_name"
    else
        _message "$red" -
        fail "$test_name"
        return 1
    fi
}

trap _print_results EXIT

# Takes the output of trap -p as parameter (trap -- 'command'
# signal) and prints the command
_print_trap_command()
{
    printf '%s' "$3"
}

# Push a command to be called on exit. The command is the argument of
# this function, it will be scheduled before all already registered
# functions. For example `push_on_exit echo good; push_on_exit echo
# bye` will print 'good' then, on the next line, 'bad'.
push_on_exit()
{
    local new_commands

    # ShellCheck suggests to put the $(trap …) between quotes to
    # prevent word splitting, but the intent here is to split. The
    # output of trap -p has the commands quoted so it fits nicely with
    # eval.
    #
    # shellcheck disable=SC2046
    new_commands="$(echo -n "$@" ';'; eval _print_trap_command $(trap -p EXIT))"

    trap -- "$new_commands" EXIT
}

# Check that a command terminates with exit code zero. For example
# `expect_true true` will pass, `expect_true false` will count as a
# failure. The script does not stop on failure.
expect_true()
{
    set +e
    "$@"
    local e=$?
    set -e

    if (( e != 0 ))
    then
        fail_count=$((fail_count + 1))
        fail "$@"
        fail "Command should have exited normally. Exit code is $e."
    else
        pass "$@"
    fi
}

# Check that a command terminates with non-zero exit code. For example
# `expect_false false` will pass, `expect_false true` will count as a
# failure. The script does not stop on failure.
expect_false()
{
    if "$@"
    then
        fail_count=$((fail_count + 1))
        fail "$@"
        fail "Command should have failed."
    else
        pass  "!" "$@"
    fi
}

# Check that the two arguments are lexicographically equal to each
# other. For example `expect_eq 'abc def' 'abc def'` pass, `expect_eq 1
# 01` fails.
expect_eq()
{
    if (( $# != 2 ))
    then
        fail_count=$((fail_count + 1))
        fail "Expected two arguments, got $#:" "$@"
        return
    fi

    if [[ "$1" = "$2" ]]
    then
        pass "'$1' = '$2'."
    else
        fail_count=$((fail_count + 1))
        fail  "'$1' is different from '$2'."
    fi
}

# Check that the two arguments are lexicographically different from
# each other. For example `expect_ne abc def` pass, `expect_ne abc
# abc` fails.
expect_ne()
{
    if (( $# != 2 ))
    then
        fail_count=$((fail_count + 1))
        fail "Expected two arguments, got $#:" "$@"
        return
    fi

    if [[ "$1" != "$2" ]]
    then
        pass "'$2' != '$1'."
    else
        fail_count=$((fail_count + 1))
        fail "'$2' != '$1'."
    fi
}

# Pass the second argument to eval then check it is lexicographically
# equal to the first argument. For example `expect_eq '123' 'echo 123'`
# pass, `expect_eq 123 123` fails.
#
# This is similar to expect_eq except that expect_eval_eq is able to
# display the expression in the logs, while expect_eq can only print
# the result.
expect_eval_eq()
{
    if (( $# != 2 ))
    then
        fail_count=$((fail_count + 1))
        fail "Expected two arguments, got $#:" "$@"
        return
    fi

    local expected
    expected="$1"

    local actual
    actual="$(eval "$2")"

    if [[ "$expected" = "$actual" ]]
    then
        pass "'$2' = '$1'."
    else
        fail_count=$((fail_count + 1))
        fail "'$2' = '$1'."
        echo "Expected: $expected"
        echo "  Actual: $actual"
    fi
}

# Usage: expect_json_eq JSON FILE, where JSON is a JSON document as a
# string, and FILE is a JSON file. The function passes if the content
# of the JSON file is identical to the provided JSON string.
expect_json_eq()
{
    if (( $# != 2 ))
    then
        fail_count=$((fail_count + 1))
        fail "Expected two arguments, got $#:" "$@"
        return
    fi

    # Uniformize the JSON representation of both arguments.

    local expected
    expected="$(echo "$1" | jq --sort-keys --compact-output .)"

    local actual
    actual="$(jq --sort-keys --compact-output . "$2")"

    if [[ "$expected" = "$actual" ]]
    then
        pass "json_eq '$1' = '$2'."
    else
        fail_count=$((fail_count + 1))
        fail "json_eq '$2'."
        echo "Expected: $expected"
        echo "  Actual: $actual"
    fi
}

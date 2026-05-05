# shellcheck shell=bash
#
# Test functions specifically designed for the current project. Source
# this script to get access to the test functions.
#

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    cat <<EOF
Usage: "$@" OPTIONS

Where OPTIONS are:
  --binary PATH
     Path to the program to test. Required.
  -h, --help
     Display this message and exit.
EOF
    exit 0
fi

# Parse the command line arguments. We need the binary of the program
# to test.
while [[ $# -ne 0 ]]
do
    arg="$1"
    shift

    case "$arg" in
        --binary)
            if [[ $# -eq 0 ]]
            then
                echo "Missing value for --binary." >&2
                exit 1
            fi

            _server_binary="$1"
            shift
            ;;
        *)
            echo "Unsupported argument '$arg'." >&2
            exit 1
            ;;
    esac
done

if [[ "${_server_binary:-}" = "" ]]
then
    echo "--binary is required."
    exit 1
fi

_test_functions_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"
_test_functions_script_dir="$(readlink --canonicalize \
                                      "$_test_functions_script_dir")"

# shellcheck source-path=SCRIPTDIR
. "$_test_functions_script_dir"/testlib.sh

# Temporary directory usable by the tests.
tmp_dir="$(mktemp --directory)"

# This is the service exposed when _server_binary is started.
_service="https://localhost:3000"

_kill_services()
{
    if [[ "${_server_pid:-}" = "" ]]
    then
        echo -e "${yellow}[ INFO ]$reset_color Server not started, no process to kill."
    else
        echo -e "${yellow}[ INFO ]$reset_color Killing server, pid='${_server_pid}'."

        kill -15 "$_server_pid" 2>/dev/null || true

        # Check that the server is not running. If it still is then we
        # kill it.
        ! kill -0 "$_server_pid" || kill -9 "$_server_pid"
    fi

    if [[ "${_container_name:-}" = "" ]]
    then
        echo -e "${yellow}[ INFO ]$reset_color Container not started, nothing to stop."
    else
        echo -e "${yellow}[ INFO ]$reset_color Stopping container '${_container_name}'."
        docker stop "$_container_name"
    fi
}

push_on_exit _kill_services

# Clean up function that removes the temporary directory if the tests
# passed and print its path if they failed.
_rm_tmp_dir()
{
    if (( fail_count == 0 ))
    then
        rm --force --recursive "$tmp_dir"
    else
        echo -e "${yellow}[ INFO ]$reset_color Temporary files are in '$tmp_dir'."

        # On the CI the user cannot easily connect to browse the
        # files, so we dump them in the terminal instead for easier
        # access.
        if [[ "${CI:-}" = "true" ]]
        then
            find "$tmp_dir" -type f \
                | while read -r f
            do
                echo "==== $f ===="
                cat "$f"
            done
        fi
    fi
}

push_on_exit _rm_tmp_dir

# Wait for a given file to contain the given regular expression, or up
# to a given timeout. If the timeout is reached the function dumps the
# content of the file as well as a provided error/second file, adds a
# failure to the test suite, and exits with an error.
_wait_poll_file()
{
    if (( $# != 4 ))
    then
        fail_count=$((fail_count + 1))
        echo -e \
             "${red}[ FAIL ]$reset_color Expected four arguments, got $#:" \
             "$@"
        return 1
    fi

    # How many seconds to wait.
    local seconds="$1"

    # The file to observe.
    local f="$2"

    # The regular expression to search in the file.
    local regex="$3"

    # Another file to dump on error.
    local error_file="$4"

    echo -e "${yellow}[ INFO ]$reset_color Waiting for '$regex' in '$f'."

    while (( seconds >= 1 ))
    do
        if grep --quiet "$regex" "$f"
        then
            return 0
        fi

        seconds=$((seconds - 1))
        sleep 1
    done

    echo -e \
         "${red}[ FAIL ]$reset_color Could not find pattern '$regex' in '$f':" \
         >&2
    cat "$f"

    if [[ -s "${error_file:-}" ]]
    then
        echo -e \
             "${red}[ FAIL ]$reset_color Error file:" \
             >&2
        cat "$error_file"
    fi

    fail_count=$((fail_count + 1))

    return 1
}

# Start the database and wait for it to be up and ready.
_container_name="test-$(echo -n "$test_name" | tr -c 'a-zA-Z0-9_.\-' '.')"
docker run --rm --name "$_container_name" \
       --env POSTGRES_PASSWORD=postgres \
       --publish 5432:5432 \
       postgres:18 \
       > "$tmp_dir"/postgres.out.txt \
       2> "$tmp_dir"/postgres.err.txt \
    &

if _wait_poll_file 60 \
                   "$tmp_dir"/postgres.out.txt \
                   "ready for start up" \
                   "$tmp_dir"/postgres.err.txt
then
    # Now that he database is up the server can start.
    "$_server_binary" \
        > "$tmp_dir"/server.out.txt \
        2> "$tmp_dir"/server.err.txt \
        &
    _server_pid=$!

    _wait_poll_file 60 \
                    "$tmp_dir"/server.out.txt \
                    'Starting the web services' \
                    "$tmp_dir"/server.err.txt
fi

# Run a request to the server using the default service and the
# default certificates. Fails if the request ends with an HTTP error
# code or if curl encounter an error, succeeds otherwise.
_do_curl()
{
    local resource="$_service/$1"
    shift

    curl --silent --show-error --fail --cacert \
         "$_test_functions_script_dir"/../certificates/localhost.crt \
         "$resource" \
         "$@"
}

# Check that _do_curl ends up with a given HTTP error code. The first
# argument is the expected error code, the rest is passed to
# _do_curl. The test will fail if the request succeeds or if the
# request ends with an error different from the expected one.
_expect_curl_error()
{
    local expected="$1"
    shift

    local tmp
    tmp="$(mktemp --tmpdir="$tmp_dir")"

    # The request must fail.
    expect_false _do_curl "$@" 2> "$tmp"

    # Ensure that there is actually an HTTP error code.
    if grep --quiet 'The requested URL returned error' "$tmp"
    then
        local actual
        actual="$(sed 's/.\+: //' "$tmp")"

        if [[ "$expected" = "$actual" ]]
        then
            echo -e "${green}[ PASS ]$reset_color Error code $expected for" "$@"
        else
            fail_count=$((fail_count + 1))
            echo -e "${red}[ FAIL ]$reset_color Wrong error code."
            echo "Expected: $expected"
            echo "  Actual: $actual"
        fi
    else
        fail_count=$((fail_count + 1))
        echo -e "${red}[ FAIL ]$reset_color Wrong error kind."
        echo "Expected: $expected"
        echo "  Actual: $(cat "$tmp")"
    fi
}

# Create an HTTP GET request with the provided arguments, and check
# that the request succeeds.
expect_get()
{
    expect_true _do_curl "$@"
}

# Create an HTTP POST request with the provided arguments, and check
# that the request succeeds.
expect_post()
{
    expect_true _do_curl "$@" -X POST
}

# Create an HTTP GET request with the provided arguments, and check
# that the request fails. The first argument is the expected HTTP
# error code, the rest is going to be passed to curl.
expect_get_error()
{
    _expect_curl_error "$@"
}

# Create an HTTP POST request with the provided arguments, and check
# that the request fails. The first argument is the expected HTTP
# error code, the rest is going to be passed to curl.
expect_post_error()
{
    _expect_curl_error "$@" -X POST
}


# shellcheck shell=bash
#
# Test functions specifically designed for the current project. Source
# this script to get access to the test functions.
#

_db_port=5432
app_port=4209

if printf '%s\n' "$@" | grep --quiet '^\(-h\|--help\)$'
then
    cat <<EOF
Usage: "$@" OPTIONS

Where OPTIONS are:
  --build-type [ debug | release ]
     The build to test. Required.
  --db-port PORT
     Port on which Postgres will listen. Default is $_db_port.
  --port PORT
     Port on which the app will listen. Default is $app_port.
  --workspace DIR
     Where to put the files produced by the tests.
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
        --build-type)
            if [[ $# -eq 0 ]]
            then
                echo "Missing value for --build-type." >&2
                exit 1
            fi

            build_type="$1"
            shift
            ;;
        --db-port)
            if [[ $# -eq 0 ]]
            then
                echo "Missing value for --db-port." >&2
                exit 1
            fi

            _db_port="$1"
            shift
            ;;
        --port)
            if [[ $# -eq 0 ]]
            then
                echo "Missing value for --port." >&2
                exit 1
            fi

            app_port="$1"
            shift
            ;;
        --workspace)
            if [[ $# -eq 0 ]]
            then
                echo "Missing value for --workspace." >&2
                exit 1
            fi

            _workspace="$1"
            shift
            ;;
        *)
            echo "Unsupported argument '$arg'." >&2
            exit 1
            ;;
    esac
done

if [[ "${build_type:-}" = "" ]]
then
    echo "--build-type is required."
    exit 1
fi

_test_functions_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"
_test_functions_script_dir="$(readlink --canonicalize \
                                      "$_test_functions_script_dir")"
repo_root="$(cd "$_test_functions_script_dir"/../; pwd)"
_server_binary="$repo_root"/target/"$build_type"/bim-business

# shellcheck source-path=SCRIPTDIR
. "$_test_functions_script_dir"/testlib.sh

# Temporary directory usable by the tests.
if [[ -z "${_workspace:-}" ]]
then
    tmp_dir="$(mktemp --directory)"
else
    tmp_dir="$(readlink --canonicalize "$_workspace")"
    tmp_dir+="/$test_name"
    mkdir --parents "$tmp_dir"
fi

_db_name=test-db
_db_user=postgres_user

# This is the service exposed when _server_binary is started.
_service="http://localhost:$app_port"

_kill_services()
{
    exit_code=$?

    if [[ "${_server_pid:-}" = "" ]]
    then
        info "Server not started, no process to kill."
    else
        info "Killing server, pid='${_server_pid}'."

        kill -15 "$_server_pid" 2>/dev/null || true

        # Check that the server is not running. If it still is then we
        # kill it.
        ! kill -0 "$_server_pid" || kill -9 "$_server_pid"
    fi

    if [[ "${_db_container_name:-}" = "" ]]
    then
        info "Container not started, nothing to stop."
    else
        info "Stopping container '${_db_container_name}'."
        docker stop "$_db_container_name"
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
        info "Temporary files are in '$tmp_dir'."

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

# Make sure we exit with a failure if the last executed command failed
# unexpectedly. This may happen due to the set -eu flags, stopping the
# scripts even though fail_count is zero. Without this function the
# other trapped functions would override the exit code and the calling
# script would not see the problem.
count_failure_on_script_error()
{
    local exit_code=$?

    if (( fail_count == 0 )) && (( exit_code != 0 ))
    then
        fail_count=1
    fi
}

push_on_exit count_failure_on_script_error

# Wait for a given file to contain the given regular expression, or up
# to a given timeout. If the timeout is reached the function dumps the
# content of the file as well as a provided error/second file, adds a
# failure to the test suite, and exits with an error.
_wait_poll_file()
{
    if (( $# != 4 ))
    then
        fail_count=$((fail_count + 1))
        fail "Expected four arguments, got $#:" "$@"
        return 1
    fi

    # How many seconds to wait.
    local seconds="$1"

    # The file to observe.
    local f="$2"

    # The regular expression to search in the file.
    local regex="$3"

    # Another file to dump on error.
    local second_file="$4"

    info "Waiting for '$regex' in '$f'."

    while (( seconds >= 1 ))
    do
        if [[ -f "$f" ]] && grep --quiet "$regex" "$f"
        then
            return 0
        fi

        seconds=$((seconds - 1))
        sleep 1
    done

    fail "Could not find pattern '$regex' in '$f':"
    cat "$f"

    if [[ -s "${second_file:-}" ]]
    then
        fail "Second file:"
        cat "$second_file"
    fi

    fail_count=$((fail_count + 1))

    return 1
}

wait_server_ready()
{
    local stdout="$1"
    local stderr="$2"

    _wait_poll_file 60 \
                    "$stdout" \
                    'Starting the web services' \
                    "$stderr"
}

start_server()
{
    _db_password=postgres_password

    # Start the database and wait for it to be up and ready.
    _db_container_name="test-$(echo -n "$test_name" \
                                    | tr -c 'a-zA-Z0-9_.\-' '.')"
    docker run --rm --name "$_db_container_name" \
           --env POSTGRES_USER="$_db_user" \
           --env POSTGRES_PASSWORD="$_db_password" \
           --env POSTGRES_DB="$_db_name" \
           --publish 5432:"$_db_port" \
           postgres:18 \
           > "$tmp_dir"/postgres.out.txt \
           2> "$tmp_dir"/postgres.err.txt \
        &

    if _wait_poll_file 60 \
                       "$tmp_dir"/postgres.err.txt \
                       "ready to accept connections" \
                       "$tmp_dir"/postgres.out.txt
    then
        cat > "$tmp_dir"/secrets.json <<EOF
{
  "db_password": "$_db_password"
}
EOF

        # Now that he database is up the server can start.
        "$_server_binary" \
            --port "$app_port" \
            --db-port "$_db_port" \
            --db-name "$_db_name" \
            --db-user "$_db_user" \
            --secrets "$tmp_dir"/secrets.json \
            > "$tmp_dir"/server.out.txt \
            2> "$tmp_dir"/server.err.txt \
            &
            _server_pid=$!

            wait_server_ready \
                "$tmp_dir"/server.out.txt \
                "$tmp_dir"/server.err.txt
    fi
}

# Run a request to the server using the default service. Fails if the
# request ends with an HTTP error code or if curl encounter an error,
# succeeds otherwise.
_do_curl()
{
    local resource="$_service/$1"
    shift

    curl --silent --show-error --fail \
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
            pass "Error code $expected for" "$@"
        else
            fail_count=$((fail_count + 1))
            fail "Wrong error code."
            echo "Expected: $expected"
            echo "  Actual: $actual"
        fi
    else
        fail_count=$((fail_count + 1))
        fail "Wrong error kind."
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
    expect_true _do_curl "$@" --request POST
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
    _expect_curl_error "$@" --request POST
}

# Run a request against the database.
expect_db()
{
    set +e
    docker exec "$_db_container_name" \
           psql \
           --dbname "$_db_name" \
           --port "$_db_port" \
           --username "$_db_user" \
           --command "$1" \
           > "$2"
    local e=$?
    set -e

    if (( e != 0 ))
    then
        fail_count=$((fail_count + 1))
        fail "$1"
        fail "Request failed. Exit code is $e."
        cat "$2" >&2
    else
        pass "$1"
    fi
}

#!/bin/bash

function _check_stat()
{
    local stat_name="$1"
    local expected="$2"
    local log="$3"

    local actual
    actual="$(grep "^ *$stat_name *|" "$log" | awk '{print $3}')"

    expect_eq "$expected" "$actual" "stat=$stat_name"
}

function check_player_stats()
{
    local user_id="$1"
    local defeats="$2"
    local kicked="$3"
    local draws="$4"
    local victories="$5"

    # tmp_dir is expected to be set by caller. This assignment exists
    # only to silence ShellCheck.
    tmp_dir="${tmp_dir:-}"

    expect_db 'select * from arena_stats
               where user_id = '"$user_id" \
                   "$tmp_dir"/stats-"$user_id".txt

    _check_stat defeats "$defeats" "$tmp_dir"/stats-"$user_id".txt
    _check_stat kicked "$kicked" "$tmp_dir"/stats-"$user_id".txt
    _check_stat draws "$draws" "$tmp_dir"/stats-"$user_id".txt
    _check_stat victories "$victories" "$tmp_dir"/stats-"$user_id".txt
}

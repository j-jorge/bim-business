#!/bin/bash

set -euo pipefail

common_name="$1"
name="$2"

mkdir --parents certificates
openssl req -noenc -days 365 -new -x509 \
        -subj "/CN=$common_name" \
        -keyout certificates/"$name".key \
        -out certificates/"$name".crt

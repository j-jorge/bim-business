#!/bin/bash

mkdir --parents /opt/bim/host/logs
/opt/bim/bin/bim-business \
    "$@" \
    >> /opt/bim/host/logs/bim-business.stdout.txt \
    2>> /opt/bim/host/logs/bim-business.stderr.txt

#!/bin/bash

set -euo pipefail

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

bim_curl admin/leads/create -X POST -H "Authorization: _"

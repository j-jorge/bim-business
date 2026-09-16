#!/bin/bash

set -euo pipefail

lead_required=1

# shellcheck source-path=SCRIPTDIR
. "$(cd "$(dirname "${BASH_SOURCE[0]}")"; pwd)"/../cli-helpers.sh

bim_curl admin/flat-client-config/list

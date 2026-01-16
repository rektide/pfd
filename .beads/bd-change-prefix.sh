#!/bin/bash

# bd-change-prefix.sh - Change beads issue prefix
# Usage: bd-change-prefix.sh <from-prefix> <to-prefix>

set -euo pipefail

FROM_PREFIX="$1"
TO_PREFIX="$2"

if [[ -z "$FROM_PREFIX" || -z "$TO_PREFIX" ]]; then
  echo "Usage: $0 <from-prefix> <to-prefix>"
  echo "Example: $0 prefork-rs pfd"
  exit 1
fi

echo "Changing prefix from '$FROM_PREFIX' to '$TO_PREFIX' in .beads/issues.jsonl"

# Generate and run jq script
jq_script='
  (.id |= sub("^'"$FROM_PREFIX"'-"; "'"$TO_PREFIX"'-")) |
  (.dependencies[]?.issue_id |= sub("^'"$FROM_PREFIX"'-"; "'"$TO_PREFIX"'-")) |
  (.dependencies[]?.depends_on_id |= sub("^'"$FROM_PREFIX"'-"; "'"$TO_PREFIX"'-"))
'

# Create backup
cp .beads/issues.jsonl .beads/issues.jsonl.bak

# Apply changes (compact output)
echo "$jq_script" | jq -c -R -f - .beads/issues.jsonl.bak > .beads/issues.jsonl

echo "Done. Backup saved to .beads/issues.jsonl.bak"
echo "You may need to restart the beads daemon: pkill -f 'bd daemon'"

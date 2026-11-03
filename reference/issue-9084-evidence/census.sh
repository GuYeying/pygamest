#!/usr/bin/env bash
# Census: count thread_local!/static_cell! TLS statics in rustpython-vm + rustpython-common.
# Run against rustpython 0.6.0-dev snapshot 23fcb8d9 (2026-09-22).
# Usage: ./census.sh [path-to-RustPython-checkout]
set -euo pipefail
RP_DIR="${1:-$(cd "$(dirname "$0")/../.." && pwd)/src/rpy/dependencies/RustPython-main}"
AWKPROG=$(mktemp)
cat > "$AWKPROG" <<'AWK'
/thread_local!|static_cell!/ && !inblock { inblock=1; depth=0; armed=0; file=FILENAME }
inblock {
  line=$0
  cnt=gsub(/{/,"{",line)-gsub(/}/,"}",line)
  depth+=cnt
  if (cnt>0) armed=1
  if (armed && $0 ~ /static [A-Za-z_][A-Za-z_0-9]*:/) n++
  if (armed && depth<=0) { print file": "n; inblock=0 }
}
AWK
total=0
find "$RP_DIR/crates/vm/src" "$RP_DIR/crates/common/src" -name '*.rs' | while read -r f; do
  awk -f "$AWKPROG" "$f"
done | awk -F': ' '$2>0 {print; s+=$2} END {print "TOTAL thread_local! statics (vm+common): " s}'
rm -f "$AWKPROG"

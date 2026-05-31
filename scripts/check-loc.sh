#!/usr/bin/env bash
set -euo pipefail
root="${1:-$(pwd)}"
limit="${PI_GPUI_LOC_LIMIT:-1000}"
status=0
while IFS= read -r -d '' file; do
  lines=$(wc -l < "$file")
  if [ "$lines" -ge "$limit" ]; then
    echo "LOC limit exceeded: $file has $lines lines (limit < $limit)" >&2
    status=1
  fi
done < <(find "$root" \
  \( -path "$root/target" -o -path "$root/node/node_modules" -o -path "$root/node/dist" -o -path "$root/.git" -o -path "$root/vendor" \) -prune \
  -o -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.mjs' \) -print0)
exit "$status"

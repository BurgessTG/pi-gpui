#!/usr/bin/env bash
set -euo pipefail
root="${1:-$(pwd)}"
status=0
patterns=("--mode rpc" "runRpcMode" "RpcClient" "modes/rpc")
for pattern in "${patterns[@]}"; do
	if grep -RIn --exclude-dir=target --exclude-dir=node_modules --exclude-dir=dist --exclude-dir=.git --exclude-dir=vendor --exclude='forbid-stock-pi-rpc.sh' -- "$pattern" "$root"; then
		echo "Forbidden stock Pi RPC reference found: $pattern" >&2
		status=1
	fi
done
exit "$status"

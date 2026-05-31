#!/usr/bin/env bash
set -euo pipefail
root="${1:-$(pwd)}"
version="${PI_GPUI_LIBNODE_VERSION:-v24.4.1}"
platform="${PI_GPUI_LIBNODE_PLATFORM:-linux-amd64}"
out_dir="$root/.libnode/$version"
mkdir -p "$out_dir"
if [ ! -f "$out_dir/libnode.so" ] && [ ! -f "$out_dir/libnode.dylib" ] && [ ! -f "$out_dir/libnode.dll" ]; then
  curl -L --fail --retry 3 \
    --url "https://github.com/alshdavid/libnode-prebuilt/releases/download/$version/libnode-$platform.tar.xz" \
    | tar -xJf - -C "$out_dir"
fi
printf '%s\n' "$out_dir"

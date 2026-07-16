#!/usr/bin/env bash
set -euo pipefail

bundle_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root_path="$(cat "$bundle_dir/ROOT_PATH")"

case "$root_path" in
  /nix/store/*) ;;
  *)
    echo "release import: invalid toolkit store path" >&2
    exit 1
    ;;
esac

if ! command -v nix >/dev/null 2>&1; then
  echo "release import: Nix is required for this Nix-only artifact" >&2
  exit 1
fi

nix --extra-experimental-features nix-command copy \
  --from "file://$bundle_dir/cache" "$root_path"
nix-store --query "$root_path" >/dev/null

printf 'Imported toolkit: %s\n' "$root_path"
printf 'Binaries: %s/bin\n' "$root_path"

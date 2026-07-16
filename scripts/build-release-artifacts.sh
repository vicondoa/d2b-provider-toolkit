#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

toolkit_link="${1:-result-toolkit}"
source_link="${2:-result-source}"
dist="${3:-dist}"
staging="$ROOT/.release-build.$$"

cleanup() {
  rm -rf -- "$staging"
}
trap cleanup EXIT

for command_name in cargo jq nix nix-store readelf sha256sum tar; do
  command -v "$command_name" >/dev/null 2>&1 || {
    echo "release build: missing command: $command_name" >&2
    exit 1
  }
done

version="$(
  cargo metadata --no-deps --format-version 1 |
    jq -r '.packages[] | select(.name == "d2b-provider-sdk") | .version'
)"
test -n "$version"
toolkit_root="$(readlink -f -- "$toolkit_link")"
test -d "$toolkit_root/bin"

mkdir -p "$staging"
bundle_name="d2b-provider-toolkit-$version-x86_64-linux-nix-closure"
bundle="$staging/$bundle_name"
mkdir -p "$bundle/cache"

for binary in "$toolkit_root"/bin/*; do
  test -f "$binary" || continue
  interpreter="$(readelf --program-headers "$binary" |
    sed -n 's/.*Requesting program interpreter: \(.*\)]/\1/p')"
  needed="$(readelf --dynamic "$binary" |
    sed -n 's/.*(NEEDED).*Shared library: \[\(.*\)\]/\1/p')"
  runpath="$(readelf --dynamic "$binary" |
    sed -n 's/.*(\(RUNPATH\|RPATH\)).*Library runpath: \[\(.*\)\]/\2/p')"
  case "$interpreter" in
    /nix/store/*) ;;
    *)
      echo "release build: $binary lacks a Nix-store ELF interpreter" >&2
      exit 1
      ;;
  esac
  test -n "$needed" || {
    echo "release build: $binary has no DT_NEEDED entries" >&2
    exit 1
  }
  case "$runpath" in
    *"/nix/store/"*) ;;
    *)
      echo "release build: $binary lacks a Nix-store ELF runpath" >&2
      exit 1
      ;;
  esac
done

nix-store --query --requisites "$toolkit_root" |
  LC_ALL=C sort -u >"$bundle/closure-paths"
grep -Fx "$toolkit_root" "$bundle/closure-paths" >/dev/null
nix --extra-experimental-features nix-command copy \
  --to "file://$bundle/cache" "$toolkit_root"

while IFS= read -r store_path; do
  store_name="${store_path#/nix/store/}"
  store_hash="${store_name%%-*}"
  narinfo="$bundle/cache/$store_hash.narinfo"
  test -f "$narinfo" || {
    echo "release build: closure path has no cache record: $store_path" >&2
    exit 1
  }
  grep -Fx "StorePath: $store_path" "$narinfo" >/dev/null
done <"$bundle/closure-paths"

printf '%s\n' "$toolkit_root" >"$bundle/ROOT_PATH"
cp scripts/import-nix-closure.sh "$bundle/import.sh"
chmod 0755 "$bundle/import.sh"
cp docs/reference/release-artifacts.md "$bundle/README.md"

rm -rf -- "$dist"
mkdir -p "$dist"
cp "$source_link/d2b-provider-toolkit-$version-source.tar.gz" "$dist/"
tar \
  --sort=name \
  --mtime="@1" \
  --owner=0 \
  --group=0 \
  --numeric-owner \
  -C "$staging" \
  -czf "$dist/$bundle_name.tar.gz" \
  "$bundle_name"

(
  cd "$dist"
  sha256sum \
    "d2b-provider-toolkit-$version-source.tar.gz" \
    "$bundle_name.tar.gz" >SHA256SUMS
)

bash scripts/check-release-artifacts.sh "$dist"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
dist="${1:-$ROOT/dist}"
scratch="$ROOT/.release-check.$$"

cleanup() {
  rm -rf -- "$scratch"
}
trap cleanup EXIT

test -f "$dist/SHA256SUMS"
(
  cd "$dist"
  sha256sum --check --strict SHA256SUMS
)

mapfile -t source_archives < <(
  find "$dist" -maxdepth 1 -type f \
    -name 'd2b-provider-toolkit-*-source.tar.gz' -printf '%f\n'
)
mapfile -t closure_archives < <(
  find "$dist" -maxdepth 1 -type f \
    -name 'd2b-provider-toolkit-*-x86_64-linux-nix-closure.tar.gz' -printf '%f\n'
)
test "${#source_archives[@]}" -eq 1
test "${#closure_archives[@]}" -eq 1
test "$(wc -l <"$dist/SHA256SUMS")" -eq 2

archive="${closure_archives[0]}"
top="${archive%.tar.gz}"
while IFS= read -r member; do
  case "$member" in
    "$top"|"$top/"*) ;;
    *)
      echo "release check: unsafe or unexpected archive member: $member" >&2
      exit 1
      ;;
  esac
  case "/$member/" in
    *"/../"*)
      echo "release check: parent traversal in archive member: $member" >&2
      exit 1
      ;;
  esac
done < <(tar -tzf "$dist/$archive")

mkdir -p "$scratch"
tar -xzf "$dist/$archive" -C "$scratch"
bundle="$scratch/$top"
test -x "$bundle/import.sh"
grep -F "Nix-only" "$bundle/README.md" >/dev/null
root_path="$(cat "$bundle/ROOT_PATH")"
case "$root_path" in
  /nix/store/*) ;;
  *) exit 1 ;;
esac
grep -Fx "$root_path" "$bundle/closure-paths" >/dev/null
test -z "$(LC_ALL=C sort -u "$bundle/closure-paths" |
  comm -3 - "$bundle/closure-paths")"

while IFS= read -r store_path; do
  case "$store_path" in
    /nix/store/*) ;;
    *) exit 1 ;;
  esac
  store_name="${store_path#/nix/store/}"
  store_hash="${store_name%%-*}"
  narinfo="$bundle/cache/$store_hash.narinfo"
  test -f "$narinfo"
  grep -Fx "StorePath: $store_path" "$narinfo" >/dev/null
done <"$bundle/closure-paths"

echo "release artifacts: checksums and Nix closure layout verified"

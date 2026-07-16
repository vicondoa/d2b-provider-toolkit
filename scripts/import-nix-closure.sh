#!/usr/bin/env bash
set -euo pipefail

bundle_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root_path="$(cat "$bundle_dir/ROOT_PATH")"

percent_encode_path() {
  local input="$1"
  local byte encoded="" hex index
  local LC_ALL=C

  for ((index = 0; index < ${#input}; index++)); do
    byte="${input:index:1}"
    case "$byte" in
      [a-zA-Z0-9.~_/-])
        encoded+="$byte"
        ;;
      *)
        printf -v hex '%%%02X' "'$byte"
        encoded+="$hex"
        ;;
    esac
  done
  printf '%s' "$encoded"
}

is_trusted_nix_user() {
  local current_user group member principal trusted_users
  local -a current_groups trusted_principals

  if ((EUID == 0)); then
    return 0
  fi

  current_user="$(id -un)"
  read -r -a current_groups <<<"$(id -Gn)"
  trusted_users="$(
    nix --extra-experimental-features nix-command \
      config show trusted-users 2>/dev/null
  )"
  read -r -a trusted_principals <<<"$trusted_users"

  for principal in "${trusted_principals[@]}"; do
    case "$principal" in
      "*"|"$current_user")
        return 0
        ;;
      @*)
        group="${principal#@}"
        for member in "${current_groups[@]}"; do
          if [[ "$member" == "$group" ]]; then
            return 0
          fi
        done
        ;;
    esac
  done
  return 1
}

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

if ! is_trusted_nix_user; then
  echo "release import: run as root or a user listed in Nix trusted-users" >&2
  echo "release import: unsigned cache import requires --no-check-sigs" >&2
  exit 1
fi

cache_uri="file://$(percent_encode_path "$bundle_dir/cache")"
nix --extra-experimental-features nix-command copy \
  --no-check-sigs \
  --from "$cache_uri" "$root_path"
nix-store --query "$root_path" >/dev/null

printf 'Imported toolkit: %s\n' "$root_path"
printf 'Binaries: %s/bin\n' "$root_path"

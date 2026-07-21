#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

metadata="$(cargo metadata --no-deps --format-version 1)"

jq -e '
  all(.packages[];
    (.name | startswith("d2b-provider-"))
    and .publish == []
    and ((.features.default // []) == [])
  )
' <<<"$metadata" >/dev/null

jq -e --arg root "$ROOT" '
  .packages[]
  | select(.name == "d2b-provider-sdk")
  | ([.dependencies[] | select(.kind == null) | .name] | sort)
      == ["d2b-contracts", "d2b-provider", "d2b-provider-toolkit", "d2b-session"]
    and all(.dependencies[] | select(.kind == null); .uses_default_features == false)
    and all(
      .dependencies[] | select(.kind == null);
      .path == ($root + "/canonical/d2b/packages/" + .name)
    )
    and any(.dependencies[];
      .name == "d2b-contracts"
      and .uses_default_features == false
      and .features == ["v2-services"]
    )
' <<<"$metadata" >/dev/null

jq -e '
  all(.packages[].dependencies[];
    .name != "d2b-client"
    and (.name | test("^(azure|azure_)") | not)
  )
' <<<"$metadata" >/dev/null

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  copied_proto="$(git ls-files -- '*.proto')"
else
  copied_proto="$(
    find . \
      \( -path ./canonical/d2b -o -path ./.cargo-home -o -path ./.targets -o -path ./target \) \
      -prune -o -name '*.proto' -print -quit
  )"
fi
if [[ -n "$copied_proto" ]]; then
  echo "distribution policy: copied protobuf source found" >&2
  exit 1
fi

author_roots=(
  crates/d2b-provider-sdk/src
  crates/d2b-provider-conformance/src
  examples/d2b-provider-azure-fake/src
  templates/d2b-provider-template/src
  templates/d2b-provider-template/examples
)

for author_root in "${author_roots[@]}"; do
  if [[ ! -d "$author_root" || ! -r "$author_root" ]]; then
    echo "distribution policy: author root missing or unreadable: $author_root" >&2
    exit 1
  fi
done

# grep exits 0 (match: policy violation), 1 (no match: ok), or >=2 (a real
# error such as an unreadable file). Fail closed on >=2 instead of letting the
# `if` swallow it as "no match".
check_author_roots_pattern() {
  local pattern="$1"
  local violation_message="$2"
  local output
  local status=0
  output="$(grep -R -n -E --include='*.rs' "$pattern" "${author_roots[@]}" 2>&1)" || status=$?
  case "$status" in
    0)
      echo "distribution policy: $violation_message" >&2
      echo "$output" >&2
      exit 1
      ;;
    1)
      return 0
      ;;
    *)
      echo "distribution policy: author root scan failed (grep exit $status)" >&2
      echo "$output" >&2
      exit 1
      ;;
  esac
}

check_author_roots_pattern \
  'BROKER_SOCKET_PATH|/run/d2b|std::(fs|net|path)|PathBuf|Unix(Stream|Listener)|Tcp(Stream|Listener)|Command::new|env::(var|vars)' \
  'provider author surface uses ambient path or process authority'

check_author_roots_pattern \
  'serde(::|_)|derive\([^)]*(Serialize|Deserialize)|prost::|protobuf::|ttrpc::' \
  'provider author surface contains a copied wire implementation'

canonical_packages=(
  packages/d2b-contracts
  packages/d2b-provider
  packages/d2b-provider-toolkit
  packages/d2b-session
)
inventory_paths="$(
  jq -r '
    .distributions[]
    | select(.id == "d2b-provider-toolkit")
    | .sourceGroups[]
  ' pins/toolkit-source-contract.json |
    while IFS= read -r group; do
      jq -r --arg group "$group" '
        .sourceGroups[]
        | select(.id == $group)
        | .files[].path
      ' pins/toolkit-source-contract.json
    done |
    sort -u
)"

if [[ -e canonical/d2b/.git ]]; then
  if ! diff -u \
    <(git -C canonical/d2b ls-files -- "${canonical_packages[@]}" | sort -u) \
    <(grep -E '^packages/(d2b-contracts|d2b-provider|d2b-provider-toolkit|d2b-session)/' <<<"$inventory_paths")
  then
    echo "distribution policy: canonical Git package inventory is incomplete" >&2
    exit 1
  fi
fi

canonical_root="$(cd canonical/d2b && pwd)"
canonical_metadata="$(
  cargo metadata \
    --manifest-path canonical/d2b/packages/Cargo.toml \
    --no-deps \
    --format-version 1
)"
while IFS= read -r target; do
  relative="${target#"$canonical_root/"}"
  if ! grep -Fqx -- "$relative" <<<"$inventory_paths"; then
    echo "distribution policy: Cargo target missing from inventory: $relative" >&2
    exit 1
  fi
done < <(
  jq -r '
    .packages[]
    | select(
        .name == "d2b-contracts"
        or .name == "d2b-provider"
        or .name == "d2b-provider-toolkit"
        or .name == "d2b-session"
      )
    | .targets[].src_path
  ' <<<"$canonical_metadata" |
    sort -u
)

echo "distribution policy: ok"

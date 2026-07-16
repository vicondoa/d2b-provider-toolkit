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

jq -e '
  .packages[]
  | select(.name == "d2b-provider-sdk")
  | ([.dependencies[] | select(.kind == null) | .name] | sort)
      == ["d2b-contracts", "d2b-provider", "d2b-provider-toolkit", "d2b-session"]
    and any(.dependencies[];
      .name == "d2b-contracts"
      and .uses_default_features == false
      and .features == ["v2-provider"]
    )
' <<<"$metadata" >/dev/null

jq -e '
  all(.packages[].dependencies[];
    .name != "d2b-client"
    and (.name | test("^(azure|azure_)") | not)
  )
' <<<"$metadata" >/dev/null

if find . -path ./canonical/d2b -prune -o -name '*.proto' -print -quit |
  grep -q .
then
  echo "distribution policy: copied protobuf source found" >&2
  exit 1
fi

echo "distribution policy: ok"

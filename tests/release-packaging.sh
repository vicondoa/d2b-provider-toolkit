#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
work="$ROOT/.release-test.$$"

cleanup() {
  rm -rf -- "$work"
}
trap cleanup EXIT

bash -n \
  "$ROOT/scripts/build-release-artifacts.sh" \
  "$ROOT/scripts/check-release-artifacts.sh" \
  "$ROOT/scripts/import-nix-closure.sh"

grep -F "scripts/build-release-artifacts.sh" \
  "$ROOT/.github/workflows/release.yml" >/dev/null
grep -F "scripts/check-release-artifacts.sh" \
  "$ROOT/.github/workflows/release.yml" >/dev/null
if grep -F 'tar -C result-toolkit' "$ROOT/.github/workflows/release.yml"; then
  echo "release packaging test: raw bin/share archive returned" >&2
  exit 1
fi

if test -n "${TOOLKIT_ROOT:-}"; then
  for binary in "$TOOLKIT_ROOT"/bin/*; do
    interpreter="$(readelf --program-headers "$binary" |
      sed -n 's/.*Requesting program interpreter: \(.*\)]/\1/p')"
    needed="$(readelf --dynamic "$binary" | grep -F '(NEEDED)' || true)"
    runpath="$(readelf --dynamic "$binary" |
      grep -E '\((RUNPATH|RPATH)\).*/nix/store/' || true)"
    case "$interpreter" in
      /nix/store/*) ;;
      *) exit 1 ;;
    esac
    test -n "$needed"
    test -n "$runpath"
  done
fi

dist="$work/dist"
top="d2b-provider-toolkit-0.1.0-x86_64-linux-nix-closure"
bundle="$work/$top"
store_hash="00000000000000000000000000000000"
store_path="/nix/store/$store_hash-d2b-provider-toolkit-0.1.0"
mkdir -p "$dist" "$bundle/cache/nar" "$work/source"
printf 'source fixture\n' >"$work/source/README"
tar -C "$work/source" -czf \
  "$dist/d2b-provider-toolkit-0.1.0-source.tar.gz" README
printf '%s\n' "$store_path" >"$bundle/ROOT_PATH"
printf '%s\n' "$store_path" >"$bundle/closure-paths"
printf 'StorePath: %s\nURL: nar/fixture.nar\n' "$store_path" \
  >"$bundle/cache/$store_hash.narinfo"
printf 'fixture\n' >"$bundle/cache/nar/fixture.nar"
cp "$ROOT/scripts/import-nix-closure.sh" "$bundle/import.sh"
chmod 0755 "$bundle/import.sh"
cp "$ROOT/docs/reference/release-artifacts.md" "$bundle/README.md"
tar -C "$work" -czf "$dist/$top.tar.gz" "$top"
(
  cd "$dist"
  sha256sum ./*.tar.gz >SHA256SUMS
)

bash "$ROOT/scripts/check-release-artifacts.sh" "$dist"
printf 'tamper\n' >>"$dist/d2b-provider-toolkit-0.1.0-source.tar.gz"
if bash "$ROOT/scripts/check-release-artifacts.sh" "$dist" >/dev/null 2>&1; then
  echo "release packaging test: checksum tampering was accepted" >&2
  exit 1
fi

extract_dir="$work/extracted path #? café"
import_bundle="$extract_dir/$top"
fake_bin="$work/fake-bin"
capture="$work/cache-uri"
mkdir -p "$import_bundle/cache" "$fake_bin"
cp "$ROOT/scripts/import-nix-closure.sh" "$import_bundle/import.sh"
printf '%s\n' "$store_path" >"$import_bundle/ROOT_PATH"
cat >"$fake_bin/nix" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case " $* " in
  *" config show trusted-users "*)
    printf '%s\n' "${FAKE_TRUSTED_USERS:-$(id -un)}"
    exit 0
    ;;
esac

printf '%s\n' "$@" >"$NIX_ALL_ARGS"
previous=""
for argument in "$@"; do
  if [[ "$previous" == "--from" ]]; then
    printf '%s' "$argument" >"$NIX_ARGS_CAPTURE"
    exit 0
  fi
  previous="$argument"
done
exit 1
EOF
cat >"$fake_bin/nix-store" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod 0755 "$fake_bin/nix" "$fake_bin/nix-store"
PATH="$fake_bin:$PATH" NIX_ARGS_CAPTURE="$capture" \
  NIX_ALL_ARGS="$work/nix-args" \
  "$import_bundle/import.sh" >/dev/null
expected_uri="file://${import_bundle// /%20}/cache"
expected_uri="${expected_uri//#/%23}"
expected_uri="${expected_uri//\?/%3F}"
expected_uri="${expected_uri//é/%C3%A9}"
test "$(cat "$capture")" = "$expected_uri"
grep -Fx -- '--no-check-sigs' "$work/nix-args" >/dev/null
if ((EUID != 0)) &&
  PATH="$fake_bin:$PATH" NIX_ARGS_CAPTURE="$capture" \
    NIX_ALL_ARGS="$work/nix-args-untrusted" FAKE_TRUSTED_USERS=root \
    "$import_bundle/import.sh" >/dev/null 2>&1
then
  echo "release packaging test: untrusted Nix user was accepted" >&2
  exit 1
fi
if grep -E '\beval\b' "$ROOT/scripts/import-nix-closure.sh"; then
  echo "release packaging test: importer must not use eval" >&2
  exit 1
fi

echo "release packaging tests: ok"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
work="$ROOT/.policy-test.$$"

cleanup() {
  # Restore permissions before removal; a leftover 0000 file/dir would
  # otherwise make cleanup itself fail.
  chmod -R u+rwx -- "$work" 2>/dev/null || true
  rm -rf -- "$work"
}
trap cleanup EXIT

fixture="$work/fixture"
mkdir -p "$fixture"

# Build a fixture tree from tracked files (fast, skips target/.git) plus the
# canonical/d2b submodule content the script's earlier checks need. This
# works whether $ROOT is a real git checkout (local dev, CI) or a candidate
# tree materialized as a throwaway git commit around a hermetic Nix source
# (the `checks.policy` flake check): both expose `canonical/d2b` as an
# untracked path, so `git ls-files` never returns its contents.
(
  cd "$ROOT"
  git ls-files -z | grep -zv '^canonical/d2b$' | tar -C "$ROOT" --null -T - -cf -
) | tar -C "$fixture" -xf -

mkdir -p "$fixture/canonical/d2b"
cp -a "$ROOT/canonical/d2b/." "$fixture/canonical/d2b/"
rm -rf "$fixture/canonical/d2b/.git"

policy_script="$fixture/scripts/check-distribution-policy.sh"

# Baseline: the untouched fixture must still pass.
bash "$policy_script" >/dev/null

# A missing author root must fail closed, not be silently treated as "no
# match found". `templates/d2b-provider-template/examples` is safe to remove
# for this fixture: it holds only auto-discovered example targets, so
# `cargo metadata` (which the script runs first) tolerates its absence and
# the failure genuinely comes from the author-roots check under test. This
# case is privilege-independent: a genuinely absent directory is absent for
# root too, so it always exercises the fail-closed path.
missing_root="$fixture/templates/d2b-provider-template/examples"
mv "$missing_root" "$missing_root.bak"
if bash "$policy_script" >"$work/missing-root.out" 2>"$work/missing-root.err"; then
  echo "distribution policy test: missing author root was silently accepted" >&2
  cat "$work/missing-root.out" "$work/missing-root.err" >&2
  exit 1
fi
grep -Fq "author root missing or unreadable" "$work/missing-root.err"
mv "$missing_root.bak" "$missing_root"

# chmod(1)-based "unreadable" fixtures are not deterministic under uid 0:
# root bypasses the discretionary access checks a mode of 0000 relies on
# (verified: `grep` on a chmod-0000 file, and `[ -r ]` on a chmod-0000
# directory, both report success for root), so these cases would either
# silently no-op or spuriously fail a from-root CI/build-sandbox run. Skip
# them explicitly under root and say so plainly rather than claim coverage
# that didn't run; the deterministic grep-exit>=2 case below still covers
# that branch unconditionally.
if ((EUID == 0)); then
  echo "distribution policy test: skipping chmod-based unreadable-root and unreadable-file cases (running as uid 0; permission bits do not restrict root, so this UID cannot exercise them deterministically)" >&2
else
  # An unreadable author root directory must fail closed too.
  chmod 0000 "$missing_root"
  if bash "$policy_script" >"$work/unreadable-root.out" 2>"$work/unreadable-root.err"; then
    echo "distribution policy test: unreadable author root was silently accepted" >&2
    chmod 0755 "$missing_root"
    cat "$work/unreadable-root.out" "$work/unreadable-root.err" >&2
    exit 1
  fi
  grep -Fq "author root missing or unreadable" "$work/unreadable-root.err"
  chmod 0755 "$missing_root"

  # A grep error inside an otherwise-readable root (e.g. one unreadable
  # file, not the whole directory) must also fail closed rather than being
  # swallowed as "no match found": grep exits 0 (match), 1 (no match), or
  # >=2 (error), and only 0/1 are meaningful "did it match" signals.
  templates_src="$fixture/templates/d2b-provider-template/src"
  unreadable_file="$templates_src/.policy-test-unreadable.rs"
  printf 'fn policy_test_marker() {}\n' >"$unreadable_file"
  chmod 0000 "$unreadable_file"
  if bash "$policy_script" >"$work/unreadable-file.out" 2>"$work/unreadable-file.err"; then
    echo "distribution policy test: an unreadable .rs file (grep exit >=2) was silently accepted" >&2
    chmod 0644 "$unreadable_file"
    cat "$work/unreadable-file.out" "$work/unreadable-file.err" >&2
    exit 1
  fi
  grep -Fq "author root scan failed" "$work/unreadable-file.err"
  chmod 0644 "$unreadable_file"
  rm -f "$unreadable_file"
fi

# Deterministic grep-exit>=2 coverage that does not depend on permission
# bits at all, so it runs identically under root or an unprivileged user:
# temporarily corrupt the fixture's own copy of the first author-roots
# pattern into an invalid extended regex (an unbalanced '('), which makes
# `grep -E` itself fail (exit 2) regardless of which uid invokes it. Restore
# the original script content from a byte-exact backup afterward.
cp "$policy_script" "$policy_script.orig"
broken_pattern_applied=0
restore_policy_script() {
  if ((broken_pattern_applied)); then
    mv "$policy_script.orig" "$policy_script"
  else
    rm -f "$policy_script.orig"
  fi
}
trap 'restore_policy_script; cleanup' EXIT

original_pattern='BROKER_SOCKET_PATH|/run/d2b|std::(fs|net|path)|PathBuf|Unix(Stream|Listener)|Tcp(Stream|Listener)|Command::new|env::(var|vars)'
# Use '#' as the sed delimiter: the pattern itself contains literal '/'.
sed -i "s#'${original_pattern}'#'('#" "$policy_script"
if ! grep -Fq "'('" "$policy_script"; then
  echo "distribution policy test: failed to inject a malformed grep pattern into the fixture script" >&2
  exit 1
fi
broken_pattern_applied=1

if bash "$policy_script" >"$work/broken-pattern.out" 2>"$work/broken-pattern.err"; then
  echo "distribution policy test: a malformed author-roots grep pattern (grep exit >=2) was silently accepted" >&2
  cat "$work/broken-pattern.out" "$work/broken-pattern.err" >&2
  exit 1
fi
grep -Fq "author root scan failed" "$work/broken-pattern.err"

restore_policy_script
broken_pattern_applied=0
trap cleanup EXIT

# The fixture must pass again once every fault is reverted, proving the
# failures above were caused by the injected fault and not fixture drift.
bash "$policy_script" >/dev/null

echo "distribution policy tests: ok"

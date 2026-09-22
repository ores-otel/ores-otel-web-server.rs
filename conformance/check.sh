#!/bin/sh
set -eu
mode=${1:-full}
case "$mode" in
  full|--full) mode=full ;;
  structural|--structural-only) mode=structural ;;
  *) echo "usage: conformance/check.sh [--full|--structural-only]" >&2; exit 2 ;;
esac
root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$root"
fail(){ echo "[runtime-config-conformance] $*" >&2; exit 1; }
checker=conformance/check-runtime-configs.rust
for file in .ores-mw.toml .ores-otel.toml .ores-rl.toml .ores-lru.toml .shared-auth.toml config/ores-middleware-stack.json "$checker"; do
  [ -f "$file" ] && [ ! -L "$file" ] || fail "$file must be a real non-symlink file"
done
[ ! -e .auth-shared.toml ] || fail "legacy .auth-shared.toml must not coexist with .shared-auth.toml"
grep -q 'conformance/check.sh' .zpkg.toml || fail ".zpkg.toml must expose the conformance gate"
for phase in post-install pre-build pre-pack pre-publish; do
  hook=".zpkg/hooks/$phase.bash"
  [ -f "$hook" ] || fail "missing Zed lifecycle hook: $hook"
  grep -q 'conformance/check.sh' "$hook" || fail "$hook must invoke conformance/check.sh"
done
[ -f .githooks/pre-push ] || fail "missing tracked Git pre-push hook"
grep -q 'conformance/check.sh' .githooks/pre-push || fail ".githooks/pre-push must invoke conformance/check.sh"
echo '[runtime-config-conformance] structural/lifecycle wiring: ok'
[ "$mode" = full ] || exit 0
tmp="${TMPDIR:-/tmp}/ores-runtime-config-check.$$"
trap 'rm -f "$tmp" "$tmp.test"' EXIT INT TERM
rustc --crate-name ores_runtime_config_conformance --edition=2021 --test "$checker" -o "$tmp.test"
"$tmp.test"
rustc --crate-name ores_runtime_config_conformance --edition=2021 -O "$checker" -o "$tmp"
"$tmp"

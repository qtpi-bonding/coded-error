#!/usr/bin/env bash
# Every cargo call goes through cargo-slot (max 2 heavy compiles machine-wide).
set -euo pipefail
cd "$(dirname "$0")/.."
SLOT="${CARGO_SLOT:-$(command -v cargo-slot || true)}"
[ -x "$SLOT" ] || SLOT=""   # CI and other machines run cargo directly

$SLOT cargo fmt --all -- --check
$SLOT cargo clippy --workspace --all-targets --all-features -- -D warnings
$SLOT cargo test --workspace --all-features
$SLOT cargo check -p coded-error --no-default-features
for f in serde grpc http mcp cli proto sqlx; do
    $SLOT cargo check -p coded-error --no-default-features --features "$f"
done

# Version ranges: test both ends of each, pinned explicitly. The lockfile
# resolves to the highest match on first generation, so never assume a version.
BAK="$(mktemp -d)/Cargo.lock"
cp Cargo.lock "$BAK"
trap 'cp "$BAK" Cargo.lock' EXIT
range_check() {  # crate, lowest version, feature(s), test target
    $SLOT cargo update -p "$1" --precise "$2"
    $SLOT cargo test -p coded-error --features "$3" --test "$4"
    $SLOT cargo update -p "$1"   # back up to the highest allowed
    $SLOT cargo test -p coded-error --features "$3" --test "$4"
}
range_check tonic 0.13.1 grpc grpc
range_check prost 0.13.5 proto proto_sqlx
range_check rmcp 1.1.0 mcp mcp

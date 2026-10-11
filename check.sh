#!/usr/bin/env bash
# Repository checks: Rust (format, lint, tests incl. the covenant in Kaspa's
# script engine) and the web client (types and tests).
set -euo pipefail
cd "$(dirname "$0")"

cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

if [[ -d web ]]; then
    (cd web && npm run lint && npm test)
fi

git diff --check

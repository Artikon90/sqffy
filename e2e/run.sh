#!/usr/bin/env bash
# Full e2e suite (Docker Compose service `e2e`, or local Linux).
# Prefer: cargo e2e   /   cargo e2e-all
set -euo pipefail

echo "==> host: $(uname -a)"
echo "==> rustc: $(rustc --version)"
echo "==> SQFFY_E2E_POSTGRES=${SQFFY_E2E_POSTGRES:-}"
echo "==> SQFFY_E2E_MYSQL=${SQFFY_E2E_MYSQL:-}"

export NO_COLOR=1
export CARGO_TERM_COLOR=never

if [[ -n "${SQFFY_E2E_POSTGRES:-}" || -n "${SQFFY_E2E_MYSQL:-}" ]]; then
  echo "==> cargo e2e-all"
  cargo e2e-all
else
  echo "==> cargo e2e (no server URLs set)"
  cargo e2e -- --nocapture
fi

echo "==> e2e OK"

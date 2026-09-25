#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(tr -d '\n' < "$ROOT/VERSION")"
OUT="${1:-$ROOT/dist}"
mkdir -p "$OUT"

git -C "$ROOT" archive --format=zip --prefix="cypht-gateway-v${VERSION}/" \
  -o "$OUT/cypht-gateway-v${VERSION}.zip" "v${VERSION}"

echo "$OUT/cypht-gateway-v${VERSION}.zip"

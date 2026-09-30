#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo
echo "========================================"
echo "  QuadQR complete Android release"
echo "  Rust .so libraries + Android AAR"
echo "========================================"
echo

"$ROOT/build-android.sh"
"$ROOT/build-aar.sh"

echo
echo "========================================"
echo "  QuadQR Android release complete"
echo "========================================"
echo
echo "Native libraries:"
echo "  $ROOT/android-libs/"
echo
echo "Android AAR:"
echo "  $ROOT/dist/quadqr-android-release.aar"
echo

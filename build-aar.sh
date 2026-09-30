#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo
echo "========================================"
echo "  QuadQR Android AAR packaging"
echo "  Uses existing android-libs/*.so"
echo "========================================"
echo

for abi in arm64-v8a armeabi-v7a x86_64; do
  file="$ROOT/android-libs/$abi/libquadqr.so"
  [[ -f "$file" ]] || { echo "[ERROR] Missing $file. Run ./build-android.sh first." >&2; exit 1; }
done

"$ROOT/scripts/run-gradle.sh" -p "$ROOT/packages/quadqr-android" clean assembleRelease
AAR="$ROOT/packages/quadqr-android/build/outputs/aar/quadqr-android-release.aar"
[[ -f "$AAR" ]] || { echo "[ERROR] Gradle finished but the AAR was not found: $AAR" >&2; exit 1; }
mkdir -p "$ROOT/dist"
cp -f "$AAR" "$ROOT/dist/quadqr-android-release.aar"

echo
echo "[OK] AAR created:"
echo "  $ROOT/dist/quadqr-android-release.aar"
echo

#!/usr/bin/env sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
DEST="$ROOT/packages/quadqr-flutter/android/src/main/jniLibs"

for ABI in arm64-v8a armeabi-v7a x86_64; do
  SRC="$ROOT/android-libs/$ABI/libquadqr.so"
  if [ ! -f "$SRC" ]; then
    echo "[ERROR] Missing $SRC. Run ./build-android.sh first." >&2
    exit 1
  fi
  mkdir -p "$DEST/$ABI"
  cp "$SRC" "$DEST/$ABI/libquadqr.so"
  echo "[OK] Flutter $ABI/libquadqr.so"
done

#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-android-libs}"
PLATFORM="${ANDROID_PLATFORM:-21}"
MODE="${2:-all}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Cargo/Rust is not installed. Install rustup first: https://rustup.rs" >&2
  exit 1
fi

if ! cargo ndk --version >/dev/null 2>&1; then
  echo "cargo-ndk is not installed. Run: cargo install cargo-ndk" >&2
  exit 1
fi

export CARGO_NDK_PLATFORM="$PLATFORM"

if [[ "$MODE" == "arm64" ]]; then
  rustup target add aarch64-linux-android
  cargo ndk -t arm64-v8a -o "$OUT_DIR" build --release -p quadqr-native
else
  rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
  cargo ndk \
    -t arm64-v8a \
    -t armeabi-v7a \
    -t x86_64 \
    -o "$OUT_DIR" \
    build --release -p quadqr-native
fi

echo
echo "QuadQR Android libraries generated in: $OUT_DIR"
find "$OUT_DIR" -name 'libquadqr.so' -print | sed 's/^/ - /'

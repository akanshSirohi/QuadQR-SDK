#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo
echo "========================================"
echo "  QuadQR Android native build"
echo "  arm64-v8a + armeabi-v7a + x86_64"
echo "========================================"
echo

command -v cargo >/dev/null 2>&1 || { echo "[ERROR] Rust/Cargo was not found. Install Rust from https://rustup.rs" >&2; exit 1; }
command -v rustup >/dev/null 2>&1 || { echo "[ERROR] rustup was not found." >&2; exit 1; }
cargo ndk --version >/dev/null 2>&1 || { echo "[ERROR] cargo-ndk is not installed. Run: cargo install cargo-ndk" >&2; exit 1; }

export CARGO_NDK_PLATFORM="${CARGO_NDK_PLATFORM:-21}"
OUT_DIR="$ROOT/android-libs"

echo "[1/3] Installing Rust Android targets if needed..."
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

echo
echo "[2/3] Building QuadQR native libraries..."
rm -rf "$OUT_DIR"
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o "$OUT_DIR" \
  build --release -p quadqr-native

echo
echo "[3/3] Verifying output..."
for abi in arm64-v8a armeabi-v7a x86_64; do
  file="$OUT_DIR/$abi/libquadqr.so"
  [[ -f "$file" ]] || { echo "[ERROR] Missing $file" >&2; exit 1; }
  echo "[OK] $abi/libquadqr.so"
done

echo
echo "Native build complete:"
echo "  $OUT_DIR"
echo

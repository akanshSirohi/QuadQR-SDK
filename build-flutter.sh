#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

printf '\n========================================\n'
printf '  QuadQR Flutter Android package setup\n'
printf '  Rust .so + Flutter FFI plugin\n'
printf '========================================\n\n'

./build-android.sh
./scripts/sync-flutter-libs.sh

if ! command -v flutter >/dev/null 2>&1; then
  echo "[WARNING] Flutter SDK was not found in PATH."
  echo "Native libraries were built and synchronized successfully."
  exit 0
fi

cd packages/quadqr-flutter
flutter pub get
flutter analyze

echo "[OK] QuadQR Flutter package is ready."

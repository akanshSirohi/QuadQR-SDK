#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

./build-flutter.sh
if ! command -v flutter >/dev/null 2>&1; then
  echo "[ERROR] Flutter SDK is required to validate a Flutter release package." >&2
  exit 1
fi

rm -rf dist/quadqr-flutter dist/quadqr-flutter.zip
mkdir -p dist/quadqr-flutter
cp -R packages/quadqr-flutter/. dist/quadqr-flutter/
rm -rf dist/quadqr-flutter/.dart_tool dist/quadqr-flutter/build
rm -rf dist/quadqr-flutter/android/.gradle dist/quadqr-flutter/example/.dart_tool dist/quadqr-flutter/example/build
rm -f dist/quadqr-flutter/pubspec.lock dist/quadqr-flutter/example/pubspec.lock
(
  cd dist/quadqr-flutter
  zip -qr ../quadqr-flutter.zip .
)
echo "[OK] $ROOT/dist/quadqr-flutter.zip"

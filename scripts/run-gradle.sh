#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRADLE_VERSION="8.11.1"
TOOLS_DIR="$ROOT/.tools"
GRADLE_HOME="$TOOLS_DIR/gradle-$GRADLE_VERSION"
GRADLE_EXE="$GRADLE_HOME/bin/gradle"

if [[ ! -x "$GRADLE_EXE" ]]; then
  echo "[INFO] Gradle $GRADLE_VERSION is not cached. Downloading it once..."
  mkdir -p "$TOOLS_DIR"
  ZIP="$TOOLS_DIR/gradle-$GRADLE_VERSION-bin.zip"
  URL="https://services.gradle.org/distributions/gradle-$GRADLE_VERSION-bin.zip"

  if command -v curl >/dev/null 2>&1; then
    curl -fL "$URL" -o "$ZIP"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$ZIP" "$URL"
  else
    echo "[ERROR] curl or wget is required to download Gradle." >&2
    exit 1
  fi

  command -v unzip >/dev/null 2>&1 || { echo "[ERROR] unzip is required." >&2; exit 1; }
  unzip -q -o "$ZIP" -d "$TOOLS_DIR"
  rm -f "$ZIP"
  chmod +x "$GRADLE_EXE"
fi

exec "$GRADLE_EXE" "$@"

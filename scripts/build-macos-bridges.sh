#!/usr/bin/env bash

set -euo pipefail

TARGET_TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-${1:-}}"

if [[ -z "$TARGET_TRIPLE" ]]; then
  echo "TAURI_ENV_TARGET_TRIPLE is required" >&2
  exit 1
fi

case "$TARGET_TRIPLE" in
  x86_64-apple-darwin|aarch64-apple-darwin|universal-apple-darwin)
    ;;
  *)
    echo "Skipping macOS bridge staging for non-macOS target: $TARGET_TRIPLE"
    exit 0
    ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_TAURI_DIR="$REPO_ROOT/src-tauri"
STAGE_DIR="$SRC_TAURI_DIR/gen/macos-frameworks"

mkdir -p "$STAGE_DIR"

find_source_dylib() {
  local dylib_name="$1"
  local -a candidates=(
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/release/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/release/$dylib_name"
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/release/deps/$dylib_name"
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/debug/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/debug/$dylib_name"
    "$SRC_TAURI_DIR/target/$TARGET_TRIPLE/debug/deps/$dylib_name"
    "$SRC_TAURI_DIR/target/release/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/release/$dylib_name"
    "$SRC_TAURI_DIR/target/release/deps/$dylib_name"
    "$SRC_TAURI_DIR/target/debug/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/debug/$dylib_name"
    "$SRC_TAURI_DIR/target/debug/deps/$dylib_name"
  )

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

stage_bridge() {
  local dylib_name="$1"
  local output_path="$STAGE_DIR/$dylib_name"
  local source_path

  source_path="$(find_source_dylib "$dylib_name")" || {
    echo "Missing prebuilt bridge dylib for $TARGET_TRIPLE: $dylib_name" >&2
    exit 1
  }

  cp "$source_path" "$output_path"
}

rm -f "$STAGE_DIR/libRainyNativeNotifications.dylib" "$STAGE_DIR/libRainyQuickDelegate.dylib"

stage_bridge "libRainyNativeNotifications.dylib"
stage_bridge "libRainyQuickDelegate.dylib"

echo "Staged macOS bridges for $TARGET_TRIPLE in $STAGE_DIR"

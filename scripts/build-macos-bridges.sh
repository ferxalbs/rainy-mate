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
TMP_DIR="$STAGE_DIR/.build"

mkdir -p "$STAGE_DIR" "$TMP_DIR"

find_source_dylib_for_target() {
  local source_target="$1"
  local dylib_name="$2"
  local -a candidates=(
    "$SRC_TAURI_DIR/target/$source_target/release/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/$source_target/release/$dylib_name"
    "$SRC_TAURI_DIR/target/$source_target/release/deps/$dylib_name"
    "$SRC_TAURI_DIR/target/$source_target/debug/Frameworks/$dylib_name"
    "$SRC_TAURI_DIR/target/$source_target/debug/$dylib_name"
    "$SRC_TAURI_DIR/target/$source_target/debug/deps/$dylib_name"
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
  local source_path=""

  if [[ "$TARGET_TRIPLE" == "universal-apple-darwin" ]]; then
    local x86_path=""
    local arm_path=""

    x86_path="$(find_source_dylib_for_target "x86_64-apple-darwin" "$dylib_name" || true)"
    arm_path="$(find_source_dylib_for_target "aarch64-apple-darwin" "$dylib_name" || true)"

    if [[ -n "$x86_path" && -n "$arm_path" ]]; then
      lipo -create -output "$output_path" "$x86_path" "$arm_path"
      return 0
    fi

    source_path="$(find_source_dylib_for_target "universal-apple-darwin" "$dylib_name" || true)"
  else
    source_path="$(find_source_dylib_for_target "$TARGET_TRIPLE" "$dylib_name" || true)"
  fi

  if [[ -z "$source_path" ]]; then
    echo "Missing prebuilt bridge dylib for $TARGET_TRIPLE: $dylib_name" >&2
    exit 1
  fi

  cp "$source_path" "$output_path"
}

rm -f "$STAGE_DIR/libRainyAutoLaunch.dylib" \
  "$STAGE_DIR/libRainyNativeNotifications.dylib" \
  "$STAGE_DIR/libRainyQuickDelegate.dylib"

stage_bridge "libRainyAutoLaunch.dylib"
stage_bridge "libRainyNativeNotifications.dylib"
stage_bridge "libRainyQuickDelegate.dylib"

echo "Staged macOS bridges for $TARGET_TRIPLE in $STAGE_DIR"

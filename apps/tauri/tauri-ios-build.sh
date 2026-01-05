#!/bin/bash
# Wrapper script to ensure rustup's cargo/rustc are used for iOS builds

set -eu pipefail

# Load nvm for node/pnpm
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Use rustup's cargo and rustc instead of Homebrew's
export CARGO=$(rustup which cargo)
export RUSTC=$(rustup which rustc)

# Ensure rustup's toolchain is in PATH
export PATH="$(dirname "$CARGO"):$PATH"

# Build command arguments
ARGS=(
  -v
  --platform "${PLATFORM_DISPLAY_NAME:?}"
  --sdk-root "${SDKROOT:?}"
  --framework-search-paths "${FRAMEWORK_SEARCH_PATHS:?}"
  --header-search-paths "${HEADER_SEARCH_PATHS:?}"
  --gcc-preprocessor-definitions "${GCC_PREPROCESSOR_DEFINITIONS:-}"
  --configuration "${CONFIGURATION:?}"
)

# Add optional FORCE_COLOR if set
if [ -n "${FORCE_COLOR:-}" ]; then
  ARGS+=("${FORCE_COLOR}")
fi

# Add ARCHS
ARGS+=("${ARCHS:?}")

# Run the Tauri iOS xcode-script command
pnpm tauri ios xcode-script "${ARGS[@]}"
EXIT_CODE=$?

# After build, copy the library to the expected location if it exists
# Tauri should do this automatically, but sometimes it doesn't
# Determine SRCROOT if not set (should be set by Xcode)
if [ -z "${SRCROOT:-}" ]; then
  SRCROOT="$(cd "$(dirname "$0")/src-tauri/gen/apple" && pwd)"
fi

# Find the library in the workspace target directory
WORKSPACE_ROOT="$(cd "$SRCROOT/../../../../.." && pwd)"
LIB_SOURCE="${WORKSPACE_ROOT}/target/aarch64-apple-ios/${CONFIGURATION}/libmy_movies_tauri.a"
LIB_DEST_DIR="${SRCROOT}/Externals/arm64/${CONFIGURATION}"
LIB_DEST="${LIB_DEST_DIR}/libapp.a"

if [ -f "$LIB_SOURCE" ] && [ ! -f "$LIB_DEST" ]; then
  mkdir -p "$LIB_DEST_DIR"
  cp "$LIB_SOURCE" "$LIB_DEST"
  echo "✅ Copied library to $LIB_DEST"
fi

exit $EXIT_CODE


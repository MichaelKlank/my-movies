#!/bin/bash
# Wrapper script for Tauri iOS builds from Xcode

set -e

# Xcode doesn't inherit shell environment - set up paths manually
export PATH="/usr/local/bin:/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"

# Load nvm for node/pnpm
export NVM_DIR="$HOME/.nvm"
if [ -s "$NVM_DIR/nvm.sh" ]; then
  \. "$NVM_DIR/nvm.sh"
else
  # Fallback: add node directly to PATH
  export PATH="/Users/klank/.nvm/versions/node/v24.12.0/bin:$PATH"
fi

# Enable corepack for pnpm
corepack enable pnpm 2>/dev/null || true

# Use rustup's cargo and rustc
export CARGO=$(rustup which cargo)
export RUSTC=$(rustup which rustc)
export PATH="$(dirname "$CARGO"):$PATH"

echo "=== Tauri iOS Build Script ==="
echo "Configuration: ${CONFIGURATION:-debug}"
echo "Platform: ${PLATFORM_DISPLAY_NAME:-iOS}"
echo "Archs: ${ARCHS:-arm64}"
echo "Using node: $(which node)"
echo "Using pnpm: $(which pnpm)"
echo "Using cargo: $CARGO"

# Verify pnpm is available
if ! command -v pnpm &> /dev/null; then
  echo "❌ ERROR: pnpm not found!"
  echo "Please symlink pnpm to /usr/local/bin or update PATH in this script"
  echo "Current PATH: $PATH"
  exit 1
fi

# Run the Tauri iOS xcode-script command
pnpm tauri ios xcode-script \
  -v \
  --platform "${PLATFORM_DISPLAY_NAME:?}" \
  --sdk-root "${SDKROOT:?}" \
  --framework-search-paths "${FRAMEWORK_SEARCH_PATHS:?}" \
  --header-search-paths "${HEADER_SEARCH_PATHS:?}" \
  --gcc-preprocessor-definitions "${GCC_PREPROCESSOR_DEFINITIONS:-}" \
  --configuration "${CONFIGURATION:?}" \
  ${ARCHS:?}

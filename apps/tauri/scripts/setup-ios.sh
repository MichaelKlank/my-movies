#!/bin/bash
# Setup script for iOS development
# Run this after cloning the repo or after `tauri ios init`

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_YML="$SCRIPT_DIR/../src-tauri/gen/apple/project.yml"

# Check if project.yml exists
if [ ! -f "$PROJECT_YML" ]; then
    echo "❌ project.yml not found. Run 'pnpm tauri ios init' first."
    exit 1
fi

# Check if already patched
if grep -q "tauri-ios-build.sh" "$PROJECT_YML"; then
    echo "✅ project.yml already patched"
    exit 0
fi

# Patch the build script path
echo "🔧 Patching project.yml to use custom build script..."

# Replace the default script with our custom one
sed -i '' 's|script: pnpm tauri ios xcode-script -v --platform ${PLATFORM_DISPLAY_NAME:?} --sdk-root ${SDKROOT:?} --framework-search-paths "${FRAMEWORK_SEARCH_PATHS:?}" --header-search-paths "${HEADER_SEARCH_PATHS:?}" --gcc-preprocessor-definitions "${GCC_PREPROCESSOR_DEFINITIONS:-}" --configuration ${CONFIGURATION:?} ${FORCE_COLOR} ${ARCHS:?}|script: ${SRCROOT}/../../../tauri-ios-build.sh|' "$PROJECT_YML"

# Verify the patch
if grep -q "tauri-ios-build.sh" "$PROJECT_YML"; then
    echo "✅ project.yml patched successfully"
else
    echo "❌ Patch failed - please check project.yml manually"
    exit 1
fi

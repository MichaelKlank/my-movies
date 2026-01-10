#!/bin/bash
# Setup script for iOS development
# Run this after cloning the repo or after `tauri ios init`

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="$SCRIPT_DIR/.."
PROJECT_YML="$TAURI_DIR/src-tauri/gen/apple/project.yml"
XCODEPROJ="$TAURI_DIR/src-tauri/gen/apple/my-movies-tauri.xcodeproj"

echo "🔧 iOS Setup Script"
echo "==================="

# Check if project.yml exists
if [ ! -f "$PROJECT_YML" ]; then
    echo "❌ project.yml not found. Run 'pnpm tauri ios init' first."
    exit 1
fi

# Check if already patched in project.yml
if grep -q "tauri-ios-build.sh" "$PROJECT_YML"; then
    echo "✅ project.yml already patched"
else
    # Patch the build script path in project.yml
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
fi

# Check if project.pbxproj needs updating
PBXPROJ="$XCODEPROJ/project.pbxproj"
if [ -f "$PBXPROJ" ]; then
    if grep -q "tauri-ios-build.sh" "$PBXPROJ"; then
        echo "✅ project.pbxproj already uses custom script"
    else
        echo "🔧 Regenerating Xcode project from project.yml..."
        
        # Delete the old xcodeproj to force regeneration
        rm -rf "$XCODEPROJ"
        echo "   Deleted old .xcodeproj"
        
        # Regenerate using xcodegen (Tauri uses this internally)
        cd "$TAURI_DIR/src-tauri/gen/apple"
        if command -v xcodegen &> /dev/null; then
            xcodegen generate
            echo "✅ Xcode project regenerated with xcodegen"
        else
            echo "⚠️  xcodegen not found. Installing via Homebrew..."
            brew install xcodegen
            xcodegen generate
            echo "✅ Xcode project regenerated with xcodegen"
        fi
        
        # Verify the regeneration worked
        if [ -f "$PBXPROJ" ] && grep -q "tauri-ios-build.sh" "$PBXPROJ"; then
            echo "✅ project.pbxproj now uses custom script"
        else
            echo "❌ Regeneration failed - project.pbxproj still has old script"
            echo "   Try: cd apps/tauri && pnpm tauri ios build --open"
            exit 1
        fi
    fi
else
    echo "⚠️  project.pbxproj not found - will be generated on first build"
fi

echo ""
echo "✅ iOS setup complete!"
echo "   Next: cd apps/tauri && pnpm tauri ios build --open"

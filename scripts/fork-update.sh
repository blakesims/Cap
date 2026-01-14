#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
APP_NAME="Cap - Development.app"
BUILD_PATH="$REPO_DIR/target/release/bundle/macos/$APP_NAME"
INSTALL_PATH="/Applications/$APP_NAME"
SIGNING_IDENTITY="cap-dev-signing"

cd "$REPO_DIR"

LOG_DIR="$HOME/Library/Logs/so.cap.desktop.dev"

usage() {
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  sync      Sync with upstream and rebase blake/stable"
    echo "  build     Build release app"
    echo "  install   Build and install to /Applications"
    echo "  launch    Launch app with proper stdout handling (for Raycast)"
    echo "  pull      Just pull latest blake/stable (for servers)"
    echo "  status    Show branch status and pending changes"
    echo ""
}

sync_upstream() {
    echo "==> Fetching upstream..."
    git fetch upstream

    echo "==> Updating main branch..."
    git checkout main
    git pull upstream main
    git push origin main

    echo "==> Rebasing blake/stable onto main..."
    git checkout blake/stable
    git rebase main
    git push origin blake/stable --force-with-lease

    echo "==> Done! blake/stable is up to date with upstream."
}

build_release() {
    echo "==> Building release..."
    pnpm tauri:build || true

    if [ ! -d "$BUILD_PATH" ]; then
        echo "Error: Build failed - $BUILD_PATH not found"
        exit 1
    fi

    echo "==> Build complete: $BUILD_PATH"
}

install_app() {
    build_release

    echo "==> Removing old app..."
    rm -rf "$INSTALL_PATH"

    echo "==> Installing to /Applications..."
    ditto "$BUILD_PATH" "$INSTALL_PATH"

    echo "==> Removing quarantine flags..."
    xattr -cr "$INSTALL_PATH" || true

    echo "==> Re-signing Spacedrive.framework..."
    if [ -d "$INSTALL_PATH/Contents/Frameworks/Spacedrive.framework" ]; then
        find "$INSTALL_PATH/Contents/Frameworks/Spacedrive.framework" -name "*.dylib" \
            -exec codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none {} \;
        codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none \
            "$INSTALL_PATH/Contents/Frameworks/Spacedrive.framework"
    fi

    echo "==> Re-signing other frameworks..."
    find "$INSTALL_PATH/Contents/Frameworks" -type f \( -name "*.dylib" -o -name "*.so" \) \
        -exec codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none {} \; 2>/dev/null || true
    find "$INSTALL_PATH/Contents/Frameworks" -type d -name "*.framework" \
        -exec codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none {} \; 2>/dev/null || true

    echo "==> Signing app bundle..."
    codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none "$INSTALL_PATH"

    echo "==> Verifying signature..."
    codesign --verify --deep --strict --verbose=2 "$INSTALL_PATH"

    echo ""
    echo "==> Installed and signed! Signature info:"
    codesign -dv "$INSTALL_PATH" 2>&1 | grep -E "^(Identifier|Authority|TeamIdentifier)="
}

launch_app() {
    local binary="$INSTALL_PATH/Contents/MacOS/Cap - Development"

    if [ ! -f "$binary" ]; then
        echo "Error: App not installed at $INSTALL_PATH"
        echo "Run '$0 install' first."
        exit 1
    fi

    mkdir -p "$LOG_DIR"

    nohup "$binary" >"$LOG_DIR/stdout.log" 2>"$LOG_DIR/stderr.log" </dev/null &
    disown

    echo "Launched Cap - Development (PID: $!)"
    echo "Logs: $LOG_DIR/"
}

pull_latest() {
    echo "==> Pulling latest blake/stable..."
    git fetch origin
    git checkout blake/stable
    git reset --hard origin/blake/stable
    echo "==> Done! Local branch now matches origin exactly."
}

show_status() {
    echo "==> Current branch:"
    git branch --show-current
    echo ""
    echo "==> Commits ahead of upstream/main:"
    git log upstream/main..HEAD --oneline 2>/dev/null || echo "(fetch upstream first)"
    echo ""
    echo "==> Local changes:"
    git status --short
}

case "${1:-}" in
    sync)
        sync_upstream
        ;;
    build)
        build_release
        ;;
    install)
        install_app
        ;;
    launch)
        launch_app
        ;;
    pull)
        pull_latest
        ;;
    status)
        show_status
        ;;
    *)
        usage
        exit 1
        ;;
esac

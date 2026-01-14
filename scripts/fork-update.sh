#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
APP_NAME="Cap - Development.app"
BUILD_PATH="$REPO_DIR/apps/desktop/src-tauri/target/release/bundle/macos/$APP_NAME"

cd "$REPO_DIR"

usage() {
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  sync      Sync with upstream and rebase blake/stable"
    echo "  build     Build release app"
    echo "  install   Build and install to /Applications"
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
    pnpm tauri:build
    echo "==> Build complete: $BUILD_PATH"
}

install_app() {
    build_release

    echo "==> Installing to /Applications..."
    if [ -d "/Applications/$APP_NAME" ]; then
        rm -rf "/Applications/$APP_NAME"
    fi
    cp -r "$BUILD_PATH" /Applications/

    echo "==> Installed! You can now open '$APP_NAME' from Applications."
}

pull_latest() {
    echo "==> Pulling latest blake/stable..."
    git fetch origin
    git checkout blake/stable
    git pull origin blake/stable
    echo "==> Done!"
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

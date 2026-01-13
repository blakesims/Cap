# Cap Build Guide

Quick-reference for building Cap locally.

## Prerequisites

### macOS (Apple Silicon / Intel)

```bash
brew install node@20 cmake rust
npm install -g pnpm@10.5.2
brew install orbstack  # or: brew install --cask docker
```

Add Node 20 to PATH if using keg-only version:
```bash
echo 'export PATH="/opt/homebrew/opt/node@20/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Windows

- Node 20+
- Rust 1.88+
- pnpm 10.5.2+
- LLVM/Clang
- VCPKG
- Docker Desktop

## Setup

```bash
pnpm install
pnpm cap-setup      # Downloads FFmpeg and native dependencies
pnpm env-setup      # Interactive .env configuration
```

## Development

### Desktop App (Tauri)

```bash
pnpm dev:desktop    # Dev mode with hot reload
```

First build takes 15-25 minutes (Rust compilation). Subsequent builds are incremental.

**Note**: Grant screen recording and microphone permissions to your Terminal app, not Cap.

### Web App (Next.js)

```bash
pnpm dev:web        # Starts Next.js + Docker (MySQL, MinIO)
```

## Release Build

```bash
pnpm tauri:build
```

Output: `apps/desktop/src-tauri/target/release/bundle/macos/Cap.app`

Install to Applications:
```bash
cp -r apps/desktop/src-tauri/target/release/bundle/macos/Cap.app /Applications/
```

## Rust-Only Changes

For faster iteration when only touching Rust code:
```bash
cd apps/desktop/src-tauri
cargo build         # Skip frontend rebuild
cargo check         # Even faster - type check only
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Turbo cache issues | `rm -rf .turbo` |
| Node version mismatch | Ensure Node 20 is active |
| Rust compile errors | Check `Cargo.toml` dependencies |
| Permission errors (macOS) | Grant permissions to Terminal, not Cap |
| IPC binding errors | Restart dev server to regenerate `tauri.ts` |

## Recordings Location

- **macOS**: `~/Library/Application Support/so.cap.desktop.dev/recordings`
- **Windows**: `%programfiles%/so.cap.desktop.dev/recordings`

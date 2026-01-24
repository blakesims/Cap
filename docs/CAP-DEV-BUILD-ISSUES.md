# Cap Development Build Issues Summary

## Goal

Run a self-built "Cap - Development" app from a personal fork as a standalone daily-driver app, launched via Raycast script.

## Current Status: BLOCKED

Multiple overlapping issues prevent this from working reliably.

---

## Issue 1: Code Signature / Permission Persistence

### Problem
Every rebuild creates a new ad-hoc code signature. macOS tracks Screen Recording permissions by signature, so each rebuild = permissions revoked = frozen permissions UI.

### What We Tried

| Attempt | Result |
|---------|--------|
| `xattr -cr` to remove quarantine | Did not help |
| Ad-hoc signing with `codesign --force --deep --sign -` | **Failed** - Spacedrive.framework has "unsealed contents" |
| Created self-signed certificate `cap-dev-signing` | Created successfully |
| Added `signingIdentity` to `tauri.conf.json` | Build now uses certificate |
| Re-signed Spacedrive.framework dylibs with same cert | Signature validates |

### Current State
Signing now works with consistent identity. Framework Team ID mismatch resolved by re-signing all dylibs.

---

## Issue 2: Frozen Permissions UI (via `open` command)

### Problem
When launched via `open "/Applications/Cap - Development.app"`, the permissions screen appears frozen - buttons are greyed out and unresponsive.

### What We Discovered
- Running binary directly from terminal **works**: `"/Applications/Cap - Development.app/Contents/MacOS/Cap - Development"`
- All permissions show as granted in logs when run from terminal
- The difference: terminal process inherits WezTerm's permissions

### What We Tried

| Attempt | Result |
|---------|--------|
| Reset TCC permissions with `tccutil reset` | Permissions reset, but UI still frozen |
| Reset `hasCompletedStartup` flag in store | Startup screen appeared but still frozen |
| Kill all processes and relaunch | Same frozen behavior |

### Root Cause (Suspected)
The app detects it doesn't have permissions when launched standalone (not inheriting from terminal), but the permission request UI doesn't work properly - possibly a Tauri/WebView issue when no TTY is attached.

---

## Issue 3: Crash When Launched from Raycast

### Problem
When launched from Raycast (either via `open` or direct binary), app crashes immediately with:

```
Termination Reason: Namespace SIGNAL, Code 6 Abort trap: 6
std::io::stdio::_print::he7e29e8d46f201cf
cap_desktop_lib::general_settings::init (general_settings.rs:249)
```

### Root Cause
The app tries to print to stdout during initialization, but Raycast doesn't provide a TTY. Rust panics when stdout is unavailable.

### This Is Different From
- The permissions issue (app doesn't even get that far)
- The signing issue (signature is valid)

---

## Issue 4: Spacedrive.framework Structure

### Problem
The embedded Spacedrive.framework has a non-standard structure that prevents proper code signing.

Error: `unsealed contents present in the root directory of an embedded framework`

### Workaround Applied
Manually re-sign all dylibs inside the framework with our certificate after build:

```bash
find "$APP_PATH/Contents/Frameworks/Spacedrive.framework" -name "*.dylib" \
  -exec codesign --force --sign "cap-dev-signing" {} \;
codesign --force --sign "cap-dev-signing" "$APP_PATH/Contents/Frameworks/Spacedrive.framework"
codesign --force --deep --sign "cap-dev-signing" "$APP_PATH"
```

---

## What Works

| Scenario | Result |
|----------|--------|
| Run binary directly from WezTerm | **Works** - inherits terminal permissions |
| Dev mode `pnpm dev:desktop` | **Works** - runs in terminal context |
| `open` from terminal | Frozen permissions UI |
| Launch from Raycast | **Crashes** - stdout panic |

---

## Potential Solutions (Not Yet Tried)

### For the stdout crash (Issue 3)
1. Modify Cap's Rust code to handle missing stdout gracefully
2. Create a wrapper script that provides a TTY
3. Use `script` or `unbuffer` to fake a TTY

### For permissions persistence (Issue 1 & 2)
1. Register the self-signed cert as a trusted developer in System Preferences
2. Use Tauri's built-in updater to "update" rather than replace the app
3. Sign with an Apple Developer certificate ($99/year)

### For the frozen UI (Issue 2)
1. Investigate why Tauri's WebView doesn't respond when launched without TTY
2. May be related to Issue 3 - the stdout panic may be happening early

---

## Files Modified

- `apps/desktop/src-tauri/tauri.conf.json` - Added `signingIdentity: "cap-dev-signing"`
- `~/Library/Application Support/so.cap.desktop.dev/store` - Modified `hasCompletedStartup` during debugging

## Certificate Created

- Name: `cap-dev-signing`
- Type: Self-Signed Root, Code Signing
- Location: Keychain Access → login → My Certificates
- Trusted for: Code Signing (Always Trust)

---

## Next Steps

The most promising path forward:

1. **Fix the stdout panic** - This is blocking Raycast launches entirely. The fix would be in `crates/desktop-lib/src/general_settings.rs` around line 249 - replace `println!` with proper logging that handles missing stdout.

2. **Then test Raycast launch** - Once the crash is fixed, see if permissions work correctly.

3. **If permissions still fail** - May need to programmatically grant permissions via TCC database (requires SIP disabled) or accept that the app must be launched from terminal.

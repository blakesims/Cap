# Fork Workflow Guide

Personal workflow for maintaining this Cap fork with custom fixes while staying synced with upstream.

## Branch Strategy

```
upstream/main ──────────────────────────────► (Cap official releases)
                    │
                    │ periodically sync
                    ▼
origin/main ────────────────────────────────► (mirror of upstream)
                    │
                    │ branch from
                    ▼
origin/blake/stable ────────────────────────► (personal fixes on top)
```

| Branch | Purpose | Tracks |
|--------|---------|--------|
| `main` | Clean mirror of upstream | `upstream/main` |
| `blake/stable` | Daily driver with personal fixes | Rebased on `main` |
| `fix/*` | PR branches for upstream contributions | One-off, delete after merge |

## Common Operations

### Sync with Upstream (do this regularly)

```bash
# Update main to mirror upstream
git checkout main
git pull upstream main
git push origin main

# Rebase stable branch onto latest
git checkout blake/stable
git rebase main
git push origin blake/stable --force-with-lease
```

### Add a Personal Fix (stays in your fork)

```bash
git checkout blake/stable
# make changes
git add .
git commit -m "My local fix for X"
git push origin blake/stable
```

### Contribute to Upstream (create a PR)

```bash
# Start fresh from upstream
git fetch upstream
git checkout -b fix/short-description upstream/main

# Make changes, commit
git add .
git commit -m "Fix: description of the fix"

# Push to your fork and create PR
git push origin fix/short-description
gh pr create --repo CapSoftware/Cap --head blakesims:fix/short-description
```

### After Your PR is Merged

```bash
# Delete the PR branch
git branch -d fix/short-description
git push origin --delete fix/short-description

# Sync to get your changes via upstream
git checkout main
git pull upstream main
git push origin main

# Rebase stable (your fix is now in main)
git checkout blake/stable
git rebase main
git push origin blake/stable --force-with-lease
```

### Handle Rebase Conflicts

```bash
# During rebase, if conflicts occur:
git status                    # See which files conflict
# Edit files to resolve conflicts
git add <resolved-files>
git rebase --continue

# If things go wrong, abort and try again:
git rebase --abort
```

## Building

### Dev Mode (quick iteration)
```bash
pnpm dev:desktop
```

### Release Build (install as app)
```bash
pnpm tauri:build
cp -r "apps/desktop/src-tauri/target/release/bundle/macos/Cap - Development.app" /Applications/
```

## Remotes Setup

```bash
# Verify remotes
git remote -v

# Should show:
# origin    https://github.com/blakesims/Cap.git (your fork)
# upstream  https://github.com/CapSoftware/Cap.git (official)

# If upstream is missing:
git remote add upstream https://github.com/CapSoftware/Cap.git
```

## Quick Reference

```bash
# See all branches
git branch -a

# See commit history
git log --oneline -10

# See what's different from upstream
git log upstream/main..blake/stable --oneline

# Discard all local changes (careful!)
git checkout -- .
```

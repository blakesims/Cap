# T001 Testing Guide

## Setup

```bash
cd ~/repos/cap-repo-fork/Cap
git pull origin blake/stable
pnpm install
pnpm dev:desktop
```

Open or create a recording with multiple segments to test deletion/cutting.

---

## S01+S02 Test Checklist

### Existing Shortcuts (Regression)

- [ ] `S` - Toggles split/scissors mode (cursor changes)
- [ ] `Space` - Toggles play/pause
- [ ] `Cmd+=` - Zooms in timeline
- [ ] `Cmd+-` - Zooms out timeline
- [ ] `Cmd+Z` - Undo
- [ ] `Cmd+Shift+Z` - Redo

### Migrated Shortcuts (from Timeline)

- [ ] `C` - Cuts clip at playhead position (creates split)
- [ ] `Backspace` - Deletes selected segment (select one first by clicking)
- [ ] `Delete` - Same as Backspace
- [ ] `Escape` - Clears selection (segment deselects)

### New Shortcuts (S01/S02)

- [ ] `I` - Sets IN point (no visual yet, but no error in console)
- [ ] `O` - Sets OUT point (no visual yet, but no error in console)
- [ ] `M` - Sets mark (no visual yet, but no error in console)
- [ ] `'` (apostrophe) - Jump to mark (playhead moves if mark was set)
- [ ] `` ` `` (backtick) - Same as apostrophe
- [ ] `Escape` - Also clears IN/OUT points (extended behavior)

### Input Focus Guard

- [ ] Click on a text input field in the editor (e.g., title field)
- [ ] Press `I`, `O`, `S`, `C` while focused on input
- [ ] **Expected**: Characters type into field, shortcuts do NOT fire
- [ ] Click outside the input field
- [ ] Press `S`
- [ ] **Expected**: Split mode toggles (shortcuts work again)

### Console Verification

Open DevTools (`Cmd+Option+I`) and watch for errors while testing:

- [ ] No errors when pressing `I`, `O`, `M`
- [ ] No errors when pressing `'` or `` ` `` (with or without mark set)
- [ ] No errors when pressing `Escape`

---

## Quick Smoke Test Sequence

1. Open editor with a recording
2. Press `Space` → video plays
3. Press `Space` → video pauses
4. Press `S` → cursor changes to scissors
5. Press `S` → cursor changes back
6. Press `C` → clip splits at playhead
7. Click a segment to select it
8. Press `Backspace` → segment deleted (if 2+ segments exist)
9. Press `Cmd+Z` → undo works
10. Press `I` then `O` then `Escape` → no errors
11. Press `M` then `'` → playhead jumps to mark position

---

## Known Limitations (Not Yet Implemented)

- **No visual indicators** for IN/OUT points or marks (S05)
- **No playhead navigation** with `h`/`l`/`w`/`b` (S03)
- **No playback speed control** with `Ctrl+J`/`Ctrl+L`/`K` (S04)
- **No IN/OUT region deletion** with `X` (S06)

---

## Troubleshooting

**Shortcuts not working at all:**
- Check DevTools console for errors
- Make sure focus is not on an input field
- Try clicking on the timeline area first

**Build errors:**
```bash
pnpm typecheck
pnpm lint
```

**Reset if needed:**
```bash
git stash
git pull origin blake/stable
pnpm install
```

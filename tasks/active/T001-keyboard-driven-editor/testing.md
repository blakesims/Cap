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

## S03 Test Checklist - Playhead Navigation

### Frame Stepping

- [ ] `h` - Playhead moves backward ~0.033s (1 frame)
- [ ] `l` - Playhead moves forward ~0.033s (1 frame)
- [ ] Hold at start, press `h` → stays at 0 (doesn't go negative)
- [ ] Hold at end, press `l` → stays at end (doesn't exceed duration)

### Second Stepping

- [ ] `Shift+h` - Playhead moves backward 1 second
- [ ] `Shift+l` - Playhead moves forward 1 second
- [ ] At 0.5s, press `Shift+h` → goes to 0 (clamps)

### Segment Boundary Jumping

- [ ] `w` - Jumps to next segment boundary (cut point)
- [ ] `b` - Jumps to previous segment boundary
- [ ] At start, press `b` → stays at 0
- [ ] At end, press `w` → stays at end

### Timeline Start/End

- [ ] `0` - Jumps to timeline start (0.0s)
- [ ] `$` (Shift+4) - Jumps to timeline end

### Quick Navigation Test

1. Press `0` → playhead at start
2. Press `$` → playhead at end
3. Press `b` → jumps to last segment boundary
4. Press `w` → back to end
5. Press `h` repeatedly → steps back frame by frame
6. Press `Shift+l` → jumps forward 1 second

---

## Known Limitations (Not Yet Implemented)

- **No visual indicators** for IN/OUT points or marks (S05)
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

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

- [x] `S` - Toggles split/scissors mode (cursor changes)
- [x] `Space` - Toggles play/pause
- [x] `Cmd+=` - Zooms in timeline
- [x] `Cmd+-` - Zooms out timeline
- [x] `Cmd+Z` - Undo
- [x] `Cmd+Shift+Z` - Redo

### Migrated Shortcuts (from Timeline)

- [x] `C` - Cuts clip at playhead position (creates split)
- [x] `Backspace` - Deletes selected segment (select one first by clicking)
- [x] `Delete` - Same as Backspace
- [x] `Escape` - Clears selection (segment deselects)

### New Shortcuts (S01/S02)

- [x] `I` - Sets IN point (no visual yet, but no error in console)
- [x] `O` - Sets OUT point (no visual yet, but no error in console)
- [x] `M` - Sets mark (no visual yet, but no error in console)
- [x] `'` (apostrophe) - Jump to mark (playhead moves if mark was set)
- [x] `` ` `` (backtick) - Same as apostrophe
- [x] `Escape` - Also clears IN/OUT points (extended behavior)

### Input Focus Guard

- [x] Click on a text input field in the editor (e.g., title field)
- [x] Press `I`, `O`, `S`, `C` while focused on input
- [x] **Expected**: Characters type into field, shortcuts do NOT fire
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

- [x] `h` - Playhead moves backward ~0.033s (1 frame)
- [x] `l` - Playhead moves forward ~0.033s (1 frame)
- [x] Hold at start, press `h` → stays at 0 (doesn't go negative)
- [x] Hold at end, press `l` → stays at end (doesn't exceed duration)

### Second Stepping

- [x] `Shift+h` - Playhead moves backward 1 second
- [x] `Shift+l` - Playhead moves forward 1 second
- [x] At 0.5s, press `Shift+h` → goes to 0 (clamps)

One issue that I'm seeing here is when the editor... let's say I... 7. Let's say that my cursor is in the middle of the viewfinder. If I move, if I press B or shift L, the cursor will continue to jump and outside of the viewport and it will in fact overlap with the UI elements that are beyond the timeline. So ideally I think what we need to do is bound the view by scrolling left and right to make sure that the cursor is within view. I believe that would be the correct decision.


### Segment Boundary Jumping

- [x] `w` - Jumps to next segment boundary (cut point)
- [x] `b` - Jumps to previous segment boundary
- [x] At start, press `b` → stays at 0
- [x] At end, press `w` → stays at end

Again, only issue here is that B and W will jump the cursor outside of the current viewport and it may even overlap with elements that are outside of the timeline view, if that makes sense.


### Timeline Start/End

- [x] `0` - Jumps to timeline start (0.0s) ✓ FIXED
- [x] `$` (Shift+4) - Jumps to timeline end ✓ FIXED

### Quick Navigation Test

1. Press `0` → playhead at start
2. Press `$` → playhead at end
3. Press `b` → jumps to last segment boundary
4. Press `w` → back to end
5. Press `h` repeatedly → steps back frame by frame
6. Press `Shift+l` → jumps forward 1 second

---

## S04 Test Checklist - Playback Speed Control

### Speed Increase (Ctrl+L)

- [ ] From paused at 1x, press `Ctrl+L` → starts playing at 2x, indicator shows "2x"
- [ ] While playing at 2x, press `Ctrl+L` → speeds up to 4x, indicator shows "4x"
- [ ] While playing at 4x, press `Ctrl+L` → speeds up to 8x, indicator shows "8x"
- [ ] While playing at 8x, press `Ctrl+L` → stays at 8x (max)

### Speed Decrease (Ctrl+J)

- [ ] While playing at 8x, press `Ctrl+J` → slows to 4x
- [ ] While playing at 4x, press `Ctrl+J` → slows to 2x
- [ ] While playing at 2x, press `Ctrl+J` → slows to 1x (normal playback with audio)
- [ ] While playing at 1x, press `Ctrl+J` → stays at 1x (min)

### Pause (K)

- [ ] While playing at any speed, press `K` → pauses AND resets speed to 1x
- [ ] Press `Ctrl+L` after `K` → starts at 2x (not resuming old speed)

### Speed Indicator

- [ ] Indicator visible in top-right when playing at 2x/4x/8x
- [ ] Indicator NOT visible when paused
- [ ] Indicator NOT visible when playing at 1x

### Audio

- [ ] At 1x speed: audio plays normally
- [ ] At 2x/4x/8x speeds: audio is silent (no Rust playback)

### Edge Cases

- [ ] Fast playback stops at end of timeline
- [ ] After stopping at end, `Ctrl+L` restarts from beginning at 2x
- [ ] Clicking timeline during fast playback continues from new position

---

## Known Limitations (Not Yet Implemented)

- **No visual indicators** for IN/OUT points or marks (S05)
- **No IN/OUT region deletion** with `X` (S06)
- **Viewport doesn't follow cursor** - navigation can move playhead outside visible area (future enhancement)

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

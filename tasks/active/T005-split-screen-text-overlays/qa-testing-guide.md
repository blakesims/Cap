# T005 QA Testing Guide

## Prerequisites

1. Run `pnpm run dev:desktop` to regenerate TypeScript bindings
2. Have a recording open in the editor

---

## Test Files

Save these JSON files locally for testing.

### `test-basic.json` - Minimal Valid Import
```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 1.0,
      "end": 5.0,
      "content": "Hello World"
    }
  ]
}
```

### `test-staggered-bullets.json` - Animated Bullet Points
```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 1.0,
      "end": 10.0,
      "content": "First point appears",
      "center": { "x": 0.3, "y": 0.3 },
      "fontSize": 48,
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    },
    {
      "start": 2.5,
      "end": 10.0,
      "content": "Second point follows",
      "center": { "x": 0.3, "y": 0.45 },
      "fontSize": 48,
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    },
    {
      "start": 4.0,
      "end": 10.0,
      "content": "Third point last",
      "center": { "x": 0.3, "y": 0.6 },
      "fontSize": 48,
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    }
  ]
}
```

### `test-split-screen.json` - Layout Changes + Text
```json
{
  "version": "1.0.0",
  "sceneChanges": [
    { "time": 0.0, "mode": "Screen" },
    { "time": 3.0, "mode": "SplitScreenRight" },
    { "time": 8.0, "mode": "Screen" }
  ],
  "textSegments": [
    {
      "start": 3.5,
      "end": 7.5,
      "content": "Key Takeaway",
      "center": { "x": 0.3, "y": 0.5 },
      "fontSize": 60,
      "fontWeight": "800",
      "fontColor": "#FFFFFF",
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.3, "value": 1.0 }
        ]
      }
    }
  ]
}
```

### `test-warnings.json` - Out-of-Range Values (Should Warn)
```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 0.0,
      "end": 5.0,
      "content": "Clamped position",
      "center": { "x": 1.5, "y": -0.2 }
    }
  ]
}
```

### `test-invalid-version.json` - Should Fail
```json
{
  "version": "2.0.0",
  "textSegments": []
}
```

### `test-invalid-time.json` - Should Fail
```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 10.0,
      "end": 5.0,
      "content": "Invalid range"
    }
  ]
}
```

---

## Test Cases

### Import Button UI

- [x] Import button visible in editor header (between folder and name)
- [x] Tooltip shows "Import timeline from JSON" on hover
- [x] Icon displays correctly (import arrow)

### File Picker

- [x] Click button → file picker opens
- [x] Filter shows "Timeline JSON (*.json)"
- [ ] Cancel picker → no action, no error
- [ ] Select non-JSON file → error toast

### Successful Import


test-basic when imported doesn't show anything.
- [x] Import `test-basic.json` → success toast "Imported 1 text segment(s)"
- [ ] Text segment appears in timeline
- [ ] Text renders in preview at correct time
- [ ] Import `test-staggered-bullets.json` → success toast "Imported 3 text segment(s)"
- [ ] Bullets appear staggered during playback
- [ ] Opacity animation works (fade in)
- [ ] Import `test-split-screen.json` → success toast with text + scene counts
- [ ] Scene track shows layout changes
- [ ] Split-screen mode activates at correct time

### Validation Warnings

- [ ] Import `test-warnings.json` → success toast + warning toast(s)
- [ ] Warning mentions "clamped" values
- [ ] Text still imports with corrected position

### Validation Errors

- [ ] Import `test-invalid-version.json` → error toast "Unsupported version"
- [ ] Import `test-invalid-time.json` → error toast about time range
- [ ] No segments imported on error

### State Management

- [ ] After import, undo/redo still works
- [ ] Editing imported text works (click segment, modify)
- [ ] Re-importing replaces previous import (Replace mode)
- [ ] Timeline selection cleared before import

### Edge Cases

- [ ] Import into project with no existing timeline → works
- [ ] Import into project with existing text → replaces text
- [ ] Import file with only sceneChanges, no text → works
- [ ] Import empty textSegments array → success (0 imported)

---

## Quick Smoke Test

1. Open any recording in editor
2. Click Import button
3. Select `test-staggered-bullets.json`
4. Verify toast shows "Imported 3 text segment(s)"
5. Play from 0s → verify bullets fade in at 1s, 2.5s, 4s
6. Done!

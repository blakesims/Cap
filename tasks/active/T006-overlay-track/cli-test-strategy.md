# T006 CLI-Based Visual Testing Strategy

## Overview

This document describes a semi-automated testing approach where:
1. Human provides a test recording (any content, known duration)
2. Claude generates test JSONs with specific expected visual outputs
3. Project config is modified directly (no UI needed)
4. CLI exports video at low quality
5. ffmpeg extracts frames at key timestamps
6. Claude visually inspects PNG frames to verify correctness

## Prerequisites

### 1. Build the CLI export tool
```bash
cd /Users/blake/repos/cap/cap-wt/cap-t005
cargo build --example export-cli -p cap-export --release
```

### 2. Ensure ffmpeg is available
```bash
which ffmpeg  # Should return a path
```

### 3. Provide a test recording
- Any `.cap` recording folder
- Minimum 20 seconds duration recommended
- Content doesn't matter (we're testing overlays, not the video itself)

---

## Test Cases

### Test 1: Basic Split Overlay
**Input JSON (inject to project-config.json):**
```json
{
  "overlaySegments": [
    {
      "start": 2.0,
      "end": 10.0,
      "overlayType": "split",
      "items": [
        { "delay": 0.5, "content": "TEST TITLE", "style": "title" },
        { "delay": 2.0, "content": "First bullet point", "style": "bullet" }
      ]
    }
  ]
}
```

**Expected at timestamps:**
| Time | Expected Visual |
|------|-----------------|
| 1.0s | Normal video (no overlay) |
| 2.5s | Split layout starting - camera sliding right |
| 3.0s | Split layout - "TEST TITLE" visible center-left |
| 4.5s | Split layout - "TEST TITLE" + "First bullet point" visible |
| 9.5s | Split layout still active |
| 10.5s | Normal video (overlay ended) |

### Test 2: FullScreen Overlay
**Input JSON:**
```json
{
  "overlaySegments": [
    {
      "start": 3.0,
      "end": 12.0,
      "overlayType": "fullScreen",
      "items": [
        { "delay": 0.0, "content": "FULLSCREEN TITLE", "style": "title" },
        { "delay": 1.5, "content": "Step one", "style": "numbered" },
        { "delay": 3.0, "content": "Step two", "style": "numbered" }
      ]
    }
  ]
}
```

**Expected at timestamps:**
| Time | Expected Visual |
|------|-----------------|
| 2.5s | Normal video |
| 3.5s | PiP camera visible + "FULLSCREEN TITLE" centered |
| 5.0s | PiP + title + "Step one" |
| 6.5s | PiP + title + both steps |
| 12.5s | Normal video |

### Test 3: Multiple Overlays (Gap Between)
**Input JSON:**
```json
{
  "overlaySegments": [
    {
      "start": 2.0,
      "end": 6.0,
      "overlayType": "split",
      "items": [{ "delay": 0.5, "content": "FIRST OVERLAY", "style": "title" }]
    },
    {
      "start": 8.0,
      "end": 14.0,
      "overlayType": "fullScreen",
      "items": [{ "delay": 0.5, "content": "SECOND OVERLAY", "style": "title" }]
    }
  ]
}
```

**Expected at timestamps:**
| Time | Expected Visual |
|------|-----------------|
| 3.0s | Split layout with "FIRST OVERLAY" |
| 7.0s | Normal video (gap between overlays) |
| 9.0s | Fullscreen with "SECOND OVERLAY" |

### Test 4: Text Positioning (Bullet vs Title)
**Input JSON:**
```json
{
  "overlaySegments": [
    {
      "start": 2.0,
      "end": 15.0,
      "overlayType": "split",
      "items": [
        { "delay": 0.5, "content": "CENTER TITLE", "style": "title" },
        { "delay": 2.0, "content": "Bullet one", "style": "bullet" },
        { "delay": 3.5, "content": "Bullet two", "style": "bullet" },
        { "delay": 5.0, "content": "Bullet three", "style": "bullet" }
      ]
    }
  ]
}
```

**Expected positioning:**
- Title: Centered horizontally and vertically in left panel
- Bullets: Left-aligned, stacked vertically with spacing

---

## Execution Workflow

### Step 1: Setup test recording
```bash
# Set the recording path
RECORDING="/path/to/test.cap"
OUTPUT_DIR="/tmp/cap-overlay-tests"
mkdir -p "$OUTPUT_DIR"
```

### Step 2: Backup original config
```bash
cp "$RECORDING/project-config.json" "$RECORDING/project-config.backup.json"
```

### Step 3: Inject test overlay (using jq or direct write)
```bash
# Example: Add overlay to existing config
# Claude will provide the specific JSON to inject
```

### Step 4: Export video
```bash
./target/release/examples/export-cli "$RECORDING" MP4 '{"fps":15,"resolution_base":{"x":640,"y":360},"compression":"Potato"}'
```

### Step 5: Extract frames at key timestamps
```bash
EXPORT_FILE=$(ls -t "$RECORDING"/*.mp4 | head -1)

# Extract frames at specific times
ffmpeg -ss 1.0 -i "$EXPORT_FILE" -frames:v 1 "$OUTPUT_DIR/test1_1.0s.png" -y
ffmpeg -ss 3.0 -i "$EXPORT_FILE" -frames:v 1 "$OUTPUT_DIR/test1_3.0s.png" -y
ffmpeg -ss 4.5 -i "$EXPORT_FILE" -frames:v 1 "$OUTPUT_DIR/test1_4.5s.png" -y
ffmpeg -ss 10.5 -i "$EXPORT_FILE" -frames:v 1 "$OUTPUT_DIR/test1_10.5s.png" -y
```

### Step 6: Claude inspects frames
```bash
# Claude reads the PNG files and verifies:
# - Correct layout (split vs fullscreen vs normal)
# - Text content visible and correct
# - Text positioning (title centered, bullets left-aligned)
# - Timing (appears/disappears at correct times)
```

### Step 7: Restore original config
```bash
cp "$RECORDING/project-config.backup.json" "$RECORDING/project-config.json"
```

---

## Visual Verification Checklist

For each extracted frame, Claude will verify:

1. **Layout Mode**
   - [ ] Normal: Full-width video, no split
   - [ ] Split: Camera on right (~40-60%), background on left
   - [ ] FullScreen: PiP camera in corner, text overlay

2. **Text Content**
   - [ ] Expected text is visible
   - [ ] No unexpected text
   - [ ] Text is readable (not cut off)

3. **Text Positioning**
   - [ ] Title: Centered in text area
   - [ ] Bullet: Left-aligned with bullet prefix
   - [ ] Numbered: Left-aligned with number prefix

4. **Transitions**
   - [ ] No jarring cuts (smooth blend between states)
   - [ ] Animation appears natural

5. **Timing**
   - [ ] Overlay starts at correct time
   - [ ] Items appear at correct delays
   - [ ] Overlay ends at correct time

---

## Failure Modes to Detect

1. **Overlay not rendering** - Frame shows normal video when overlay expected
2. **Wrong layout** - Split when fullscreen expected, or vice versa
3. **Missing text** - Expected text not visible
4. **Wrong positioning** - Text in wrong location
5. **Wrong timing** - Overlay starts/ends at wrong time
6. **Items not appearing** - Item delays not working
7. **Stuck overlay** - Overlay continues past end time
8. **Camera position wrong** - Camera on left instead of right in split

---

## Notes

- Export at "Potato" quality (0.04 bpp) for fast iteration
- Use 15 fps to reduce frame count while maintaining timing accuracy
- PNG extraction is lossless - good for visual inspection
- Each test should be independent (restore config between tests)

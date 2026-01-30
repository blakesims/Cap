# T006 QA Testing - Semi-Automated Approach

## CLI Export Tool

There's a CLI export example at `crates/export/examples/export-cli.rs`.

### Build the CLI
```bash
cd /Users/blake/repos/cap/cap-wt/cap-t005
cargo build --example export-cli -p cap-export
```

### Usage
```bash
# Export to MP4 (low quality for fast testing)
./target/debug/examples/export-cli "/path/to/recording.cap" MP4 '{"fps":15,"resolution_base":{"x":640,"y":360},"compression":"Potato"}'

# Export to GIF
./target/debug/examples/export-cli "/path/to/recording.cap" GIF '{"fps":10,"resolution_base":{"x":320,"y":180}}'
```

### Export Settings Reference

**MP4 Compression levels:**
- `Maximum` - 0.3 bpp (highest quality)
- `Social` - 0.15 bpp
- `Web` - 0.08 bpp
- `Potato` - 0.04 bpp (fastest, lowest quality - good for testing)

---

## What Each Test Should Look Like

### test-basic-v1.json
- **Time 0-1s:** Normal video (no text)
- **Time 1-5s:** "Hello World" text centered on screen
- **Time 5s+:** Normal video (no text)

### test-split-v1.json
- **Time 0-2s:** Normal screen recording
- **Time 2-8s:** Split-screen mode - camera on RIGHT (40%), background on LEFT (60%)
- **Time 2.5-7.5s:** "Key Point" text fades in on left side
- **Time 8s+:** Back to normal screen recording

### test-overlay-split.json (v2.0.0)
- **Time 0-2s:** Normal video
- **Time 2-12s:** Split-screen - camera RIGHT, background LEFT
- **Time 2.5s:** "Introduction" title fades/slides in (centered on left)
- **Time 4s:** "First key point" bullet appears
- **Time 5.5s:** "Second key point" bullet appears
- **Time 7s:** "Third key point" bullet appears
- **Time 12s+:** Back to normal

### test-overlay-fullscreen.json (v2.0.0)
- **Time 0-3s:** Normal video
- **Time 3-15s:** PiP camera in corner, text overlay on full screen
- **Time 3s:** "Three Steps" title appears (centered)
- **Time 4.5s:** "Step one - Setup" (numbered) appears
- **Time 6s:** "Step two - Configure" appears
- **Time 7.5s:** "Step three - Deploy" appears

### test-overlay-multiple.json (v2.0.0)
- **Time 0-2s:** Normal video
- **Time 2-8s:** Split overlay #1
- **Time 8-10s:** Normal video (gap between overlays)
- **Time 10-18s:** Fullscreen overlay #2

---

## Semi-Automated Testing Script

Create a test script that:
1. Copies a test JSON to the recording's project folder
2. Exports at low quality
3. Extracts key frames with ffmpeg
4. Opens frames for visual inspection

```bash
#!/bin/bash
# qa-test.sh - Semi-automated overlay testing

RECORDING_PATH="$1"
TEST_JSON="$2"
OUTPUT_DIR="/tmp/cap-qa-test"

mkdir -p "$OUTPUT_DIR"

# Step 1: Copy project-config with overlay (manual step - import via UI first)
echo "1. Import $TEST_JSON via Cap UI"
echo "2. Press Enter when done..."
read

# Step 2: Export low-quality video
echo "Exporting video..."
./target/debug/examples/export-cli "$RECORDING_PATH" MP4 '{"fps":15,"resolution_base":{"x":640,"y":360},"compression":"Potato"}'

# Step 3: Find the exported file
EXPORT_FILE=$(ls -t "$RECORDING_PATH"/*.mp4 2>/dev/null | head -1)
if [ -z "$EXPORT_FILE" ]; then
    echo "No export found!"
    exit 1
fi
echo "Found export: $EXPORT_FILE"

# Step 4: Extract key frames (every 1 second)
echo "Extracting frames..."
ffmpeg -i "$EXPORT_FILE" -vf "fps=1" "$OUTPUT_DIR/frame_%03d.png" -y

# Step 5: Open frames for inspection
echo "Opening frames in Finder..."
open "$OUTPUT_DIR"

echo "Done! Check frames for:"
echo "- Frame 2-3: Should show split-screen starting"
echo "- Frame 3-7: Should show text items appearing"
echo "- Frame 8+: Should show normal video"
```

---

## Quick Manual Inspection via ffmpeg

Extract specific timestamps:
```bash
# Extract frame at 2.5 seconds (when split should start)
ffmpeg -ss 2.5 -i output.mp4 -frames:v 1 frame_2.5s.png

# Extract frame at 5 seconds (when text should be visible)
ffmpeg -ss 5 -i output.mp4 -frames:v 1 frame_5s.png

# Extract frames every second
ffmpeg -i output.mp4 -vf "fps=1" frame_%02d.png
```

---

## Current Known Issue: Import vs UI Creation

**Problem identified:** Importing v2.0.0 JSON stores data to `overlay_segments`, but:
1. The Overlay Track visibility is controlled by a user toggle (`trackState().overlay`)
2. It doesn't auto-show when overlay_segments exist
3. The rendering pipeline converts overlays → scene+text at render time

**Result:** Import works (video renders correctly), but Overlay Track doesn't appear unless manually enabled in Track Manager.

**Workaround for testing:**
1. Import JSON
2. Click the track manager icon (hamburger menu in timeline)
3. Enable "Overlay" track
4. Track should now show the imported segments

---

## Test Checklist

| Test File | Import Works | Track Shows | Renders Correctly | Notes |
|-----------|--------------|-------------|-------------------|-------|
| test-basic-v1.json | [ ] | N/A (text track) | [ ] | |
| test-split-v1.json | [ ] | N/A (scene track) | [ ] | |
| test-overlay-split.json | [ ] | [ ] | [ ] | Enable overlay track manually |
| test-overlay-fullscreen.json | [ ] | [ ] | [ ] | |
| test-overlay-multiple.json | [ ] | [ ] | [ ] | |

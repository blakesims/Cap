# ISSUE-01: Multi-Segment Recording Data Loss

**Status:** OPEN - Not Started
**Severity:** Critical
**Component:** `crates/recording/src/recovery.rs`, `crates/recording/src/studio_recording.rs`
**Affected Versions:** 0.4.3+
**Reporter:** blake
**Date:** 2026-01-13

---

## Changelog

| Date | Author | Change |
|------|--------|--------|
| 2026-01-13 | blake | Initial bug report created |

---

## Summary

When a studio recording contains multiple segments (from pause/resume operations), segments in the middle of the recording lose their video data during the recovery/remux process. Only the first ~2 and last ~2 segments are preserved; all middle segments lose their `display/` and `camera/` folders.

## Reproduction Steps

1. Start a studio recording in Cap 0.4.3+
2. Pause and resume the recording multiple times (creating 10+ segments)
3. Stop the recording
4. Open the editor

**Expected:** All segments' video data is preserved and playable
**Actual:** Middle segments (e.g., segments 2-9 out of 12) lose video data; editor shows gaps/jumps

## Evidence

### Test Recording Details

- **Recording:** `DELL U2723QE (Area) 2026-01-13 01.40 PM.cap`
- **Location:** `~/Library/Application Support/so.cap.desktop.dev/recordings/`
- **Total Segments Created:** 12 (segment-0 through segment-11)
- **Segments With Video:** 4 (segment-0, segment-1, segment-10, segment-11)
- **Segments Missing Video:** 8 (segment-2 through segment-9)

### File System State After Recording

```
segments/
├── segment-0/   (6 items) ← Has display.mp4, camera.mp4 ✓
├── segment-1/   (6 items) ← Has display.mp4, camera.mp4 ✓
├── segment-2/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-3/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-4/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-5/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-6/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-7/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-8/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-9/   (4 items) ← Only audio-input.m4a, cursor.json ✗
├── segment-10/  (6 items) ← Has display.mp4, camera.mp4 ✓
└── segment-11/  (6 items) ← Has display.mp4, camera.mp4 ✓
```

### Recording Metadata (recording-meta.json)

Only 4 segments exist in the final metadata:

```json
{
  "segments": [
    { "display": { "path": "content/segments/segment-0/display.mp4" } },
    { "display": { "path": "content/segments/segment-1/display.mp4" } },
    { "display": { "path": "content/segments/segment-10/display.mp4" } },
    { "display": { "path": "content/segments/segment-11/display.mp4" } }
  ],
  "status": { "status": "Complete" }
}
```

### Log Evidence

**All 12 segments were recorded successfully** (from recording-logs.log):

```
# All segments initialized video encoder:
segment{index=0}:screen-out: Initialized segmented video encoder...
segment{index=1}:screen-out: Initialized segmented video encoder...
segment{index=2}:screen-out: Initialized segmented video encoder...
...
segment{index=11}:screen-out: Initialized segmented video encoder...

# All 12 pipelines shut down properly:
06:47:39 - pipeline shutdown (segment-0)
06:54:07 - pipeline shutdown (segment-1)
06:55:21 - pipeline shutdown (segment-2)
06:56:45 - pipeline shutdown (segment-3)
06:58:01 - pipeline shutdown (segment-4)
07:00:14 - pipeline shutdown (segment-5)
07:02:02 - pipeline shutdown (segment-6)
07:04:14 - pipeline shutdown (segment-7)
07:08:51 - pipeline shutdown (segment-8)
07:10:50 - pipeline shutdown (segment-9)
07:11:40 - pipeline shutdown (segment-10)
07:16:45 - pipeline shutdown (segment-11)
```

**Error -67 occurred for all segments** (consistent, not the differentiator):

```
WARN cap_recording::output_pipeline::core: Muxer streams had failure: Unknown error: -67
```

---

## Root Cause Analysis

### Primary Hypothesis: Alphabetical Directory Sorting Bug

The recovery process in `analyze_incomplete()` uses filesystem directory listing:

```rust
// crates/recording/src/recovery.rs:133-139
let mut segment_dirs: Vec<_> = std::fs::read_dir(&segments_dir)
    .ok()?
    .filter_map(|e| e.ok())
    .filter(|e| e.path().is_dir())
    .collect();

segment_dirs.sort_by_key(|e| e.file_name());
```

**Problem:** `sort_by_key(|e| e.file_name())` sorts alphabetically:
- `segment-0`
- `segment-1`
- `segment-10`  ← Comes before segment-2!
- `segment-11`
- `segment-2`
- `segment-3`
- ...
- `segment-9`

Then the code assigns indices based on enumeration order:

```rust
// Line 141
for (index, segment_entry) in segment_dirs.iter().enumerate() {
```

This means:
- `segment-0` → index 0 ✓
- `segment-1` → index 1 ✓
- `segment-10` → index 2 ✗ (should be 10)
- `segment-11` → index 3 ✗ (should be 11)
- `segment-2` → index 4 ✗ (should be 2)
- ...

### Secondary Factor: Recovery Overwrites Original Metadata

The `build_recovered_meta()` function creates new metadata from `recoverable_segments` only:

```rust
// crates/recording/src/recovery.rs:774-876
let segments: Vec<MultipleSegment> = recording
    .recoverable_segments
    .iter()
    .map(|seg| { ... })
    .collect();
```

If segments aren't in `recoverable_segments`, they're excluded from the final metadata.

### Tertiary Factor: Directory Deletion After Remux

After remuxing, display/camera directories are deleted:

```rust
// crates/recording/src/recovery.rs:526-531
let display_dir = segment_dir.join("display");
if display_dir.exists()
    && let Err(e) = std::fs::remove_dir_all(&display_dir)
{
    debug!("Failed to clean up display dir {:?}: {e}", display_dir);
}
```

If index mapping is wrong, this could delete wrong segment's directories.

---

## Affected Code Paths

### Key Files

| File | Function | Issue |
|------|----------|-------|
| `crates/recording/src/recovery.rs:133-139` | Directory sorting | Alphabetical sort causes wrong order |
| `crates/recording/src/recovery.rs:141` | Index assignment | Uses enumeration index, not parsed segment number |
| `crates/recording/src/recovery.rs:469-764` | `recover()` | Processes segments with potentially wrong indices |
| `crates/recording/src/recovery.rs:766-876` | `build_recovered_meta()` | Rebuilds metadata from recoverable_segments only |

### Data Flow

```
1. Recording creates segments: segment-0, segment-1, ..., segment-11
   └── Each has display/, camera/, audio-input.m4a, cursor.json

2. stop_recording() saves initial metadata with all 12 segments
   └── Paths point to display/ directories (not .mp4 files)

3. needs_fragment_remux() returns true
   └── Triggers remux_fragmented_recording()

4. analyze_incomplete() scans filesystem
   └── BUG: Alphabetical sort messes up segment order
   └── BUG: Index from enumeration doesn't match folder name

5. recover() processes segments
   └── Remuxes with wrong index mapping
   └── Deletes display/ directories based on wrong indices

6. build_recovered_meta() creates new metadata
   └── Only includes segments that were "recoverable"
   └── Overwrites original metadata

7. Result: Middle segments lost, metadata incomplete
```

---

## Proposed Fix

### Fix 1: Natural Sort for Segment Directories

Replace alphabetical sort with natural numeric sort:

```rust
// Before:
segment_dirs.sort_by_key(|e| e.file_name());

// After:
segment_dirs.sort_by_key(|e| {
    let name = e.file_name().to_string_lossy().to_string();
    if let Some(idx_str) = name.strip_prefix("segment-") {
        idx_str.parse::<u32>().unwrap_or(u32::MAX)
    } else {
        u32::MAX
    }
});
```

### Fix 2: Parse Segment Index from Folder Name

Don't use enumeration index; parse from folder name:

```rust
// Before:
for (index, segment_entry) in segment_dirs.iter().enumerate() {

// After:
for segment_entry in &segment_dirs {
    let folder_name = segment_entry.file_name().to_string_lossy();
    let index: u32 = folder_name
        .strip_prefix("segment-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
```

### Fix 3: Don't Overwrite Original Metadata

Preserve original segment list; only update paths for successfully remuxed segments:

```rust
// Instead of rebuilding metadata from scratch,
// update existing segments in place
```

---

## Testing Plan

1. **Unit Test:** Create test with 12 segment folders named segment-0 through segment-11
2. **Verify Sorting:** Assert natural numeric sort order
3. **Integration Test:** Record with 10+ pause/resume cycles, verify all segments preserved
4. **Regression Test:** Ensure existing single-segment recordings still work

---

## Related Issues

- **Error -67:** Occurs on all segments but doesn't seem to be the root cause of data loss. Likely an AVFoundation finalization warning that should be investigated separately.

---

## Workarounds

### Manual Recovery (if raw .m4s files exist)

If the original `display/` folders weren't deleted yet:

```bash
cd "/path/to/recording.cap/content/segments/segment-X/display/"
ffmpeg -i "master.m3u8" -c copy ../display.mp4
```

### For This Specific Recording

The display folders for segments 2-9 have already been deleted. The video data is **not recoverable** unless there's a backup or Time Machine snapshot.

---

## References

- Recording logs: `~/Library/Application Support/so.cap.desktop.dev/recordings/DELL U2723QE (Area) 2026-01-13 01.40 PM.cap/recording-logs.log`
- Recovery code: `crates/recording/src/recovery.rs`
- Studio recording: `crates/recording/src/studio_recording.rs`

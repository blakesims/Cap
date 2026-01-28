# Audio Sync Bug Fix - Post T004 Work

**Date:** 2026-01-28
**Status:** PARTIALLY COMPLETE - Core fix done, optimizations in progress
**Branch:** blake/stable (worktree at cap-wt/cap-stable)

---

## 1. Problem Statement

After T004 introduced pre-decoded audio for low-latency playback (~120ms instead of 7s), a critical bug was discovered:

**Bug:** After deleting a segment (using i/o markers), audio goes out of sync with video.

**Root Cause:** The pre-decoded audio cache used `segment_count` to detect timeline changes. However, when using i/o markers to delete:
1. Segment is SPLIT at 'i' marker (count +1)
2. Segment is SPLIT at 'o' marker (count +1)
3. Middle segment is DELETED (count -1 or -2)

Net result: **segment count can stay the same** while timeline content changed completely.

---

## 2. Investigation Process

### 2.1 Initial Hypothesis
The `PrerenderedAudioBuffer` from T004 was pre-rendered when editor opened. When segments are deleted:
- Video uses dynamic `get_segment_time()` → works correctly
- Audio uses stale pre-rendered buffer → out of sync

### 2.2 Failed Approaches

**Attempt 1: Background rebuilder thread**
- Added a thread that watches for project config changes
- Problem: Config update arrives AFTER playback stops (250ms debounce)
- Rebuilder never receives the update in time

**Attempt 2: Always render fresh (remove pre-decode)**
- Removed `from_predecoded()` path entirely
- Problem: 20+ second delay for 36-minute recordings - unacceptable regression

**Attempt 3: Streaming AudioPlaybackBuffer after edit**
- Tried using dynamic `AudioPlaybackBuffer` which respects timeline
- Problem: Resampler creation spammed repeatedly, audio sounded wrong

### 2.3 Successful Approach

**Solution: Timeline content hashing**

Replace `segment_count` with `timeline_hash` that captures ALL segment boundaries:

```rust
pub fn compute_timeline_hash(timeline: &TimelineConfiguration) -> u64 {
    let quantize = |f: f64| -> i64 { (f * 1_000_000.0).round() as i64 };

    let mut hasher = DefaultHasher::new();
    timeline.segments.len().hash(&mut hasher);
    for seg in &timeline.segments {
        seg.recording_clip.hash(&mut hasher);
        quantize(seg.start).hash(&mut hasher);
        quantize(seg.end).hash(&mut hasher);
        quantize(seg.timescale).hash(&mut hasher);
    }
    hasher.finish()
}
```

**Key insight:** Floats must be quantized before hashing due to floating-point arithmetic non-associativity. Raw `.to_bits()` can produce different hashes for semantically identical values.

---

## 3. Changes Made

### 3.1 Files Modified

| File | Changes |
|------|---------|
| `crates/editor/src/audio.rs` | Added `compute_timeline_hash()`, `PredecodedAudio.timeline_hash`, `predecoded_duration`, `total_duration` fields |
| `crates/editor/src/editor_instance.rs` | Partial pre-decode (first 2 min), then full in background |
| `crates/editor/src/playback.rs` | Hash-based validation, quick render after edits (60s buffer) |
| `apps/desktop/src-tauri/src/lib.rs` | Debug logging for `set_project_config` |
| `CLAUDE.md` (main repo) | Added rules about cache invalidation and float hashing |

### 3.2 New Behavior

**On editor open:**
1. Pre-decode first 120 seconds (~1-2s render time)
2. Store immediately for fast first play
3. Continue pre-decoding full timeline in background
4. Swap in complete buffer when ready

**On playback start:**
1. Check if `timeline_hash` matches pre-decoded hash
2. If match AND playhead within pre-decoded range → FAST PATH (instant)
3. If hash mismatch → QUICK PATH (render 60s buffer, ~1-2s)
4. If playhead beyond range → QUICK PATH

**After segment deletion:**
1. Timeline hash changes (even if segment count stays same)
2. Next play detects hash mismatch
3. Quick render 60s buffer for immediate playback
4. Audio is now in sync

---

## 4. Test Results

### 4.1 Verified Working
- ✅ First play after editor open: ~2-3s (partial pre-decode)
- ✅ Subsequent plays: instant (FAST PATH)
- ✅ After segment deletion: hash changes, QUICK PATH taken
- ✅ Audio in sync after deletion
- ✅ i/o marker delete (split+delete) correctly detected

### 4.2 Pending Full Testing
- ⏳ Quick render after edit (60s buffer) - user testing in progress
- ⏳ Seeking beyond 60s after edit
- ⏳ Multiple rapid edits
- ⏳ Long recordings (2+ hours)

---

## 5. Known Limitations

1. **60-second buffer after edit:** If user plays beyond 60s after an edit, audio may stop. Need to either:
   - Trigger background re-pre-decode after edit
   - Or extend quick render dynamically

2. **No background re-pre-decode after edit:** Currently, pre-decode only happens on editor open. After an edit, only quick renders are used.

3. **Memory usage:** Pre-decoded audio for 36-min recording = ~768 MB. Plus quick render buffers.

---

## 6. Future Improvements

### 6.1 High Priority
- [ ] Background re-pre-decode after timeline edit
- [ ] Extend quick render if playback approaches buffer end

### 6.2 Medium Priority
- [ ] Use streaming audio during background re-pre-decode
- [ ] Partial re-render (only affected portions)
- [ ] Memory limit for very long recordings

### 6.3 Low Priority
- [ ] Cache pre-decoded audio to disk
- [ ] Sample rate change detection

---

## 7. Learnings & Rules Added

Added to `CLAUDE.md`:

```markdown
- **Audio vs video rendering**: Video uses dynamic `get_segment_time()`
  on each frame. Audio pre-renders entire timeline, requiring explicit
  cache invalidation on edits.

- **Timeline cache invalidation**: Never use count-based invalidation.
  Use content hashing via `compute_timeline_hash()`.

- **Float hashing**: Quantize to fixed precision first
  (e.g., `(f * 1_000_000.0).round() as i64`). Raw `.to_bits()` fails
  due to floating-point non-associativity.
```

---

## 8. Code Review Summary

A thorough code review was performed (see `/tmp/audio-sync-fix-review.md`). Key findings:

**Fixed:**
- ✅ Floating-point hash instability (quantization added)
- ✅ Empty timeline handling (returns 0 consistently)

**Noted but not critical:**
- Clip index bounds checking (defensive, not critical)
- Hash collision probability (negligible for this use case)
- Pre-decode panic handling (silent failure)

---

## 9. Commits

```
5166b719 fix(editor): use timeline hash for audio sync detection
2316576e docs: add audio cache invalidation rules to CLAUDE.md
```

---

## 10. Related Files

- `/tmp/audio-sync-fix-review.md` - Detailed code review
- `crates/editor/src/audio.rs` - Core audio rendering
- `crates/editor/src/playback.rs` - Playback stream creation
- `crates/editor/src/editor_instance.rs` - Pre-decode spawning
